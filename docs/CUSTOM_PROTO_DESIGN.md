# Custom Proto Definition Support - Design Document

## Current Architecture

The recorder currently uses **hardcoded** protobuf definitions:
- Proto files: `proto/sensor_data.proto`
- Compiled at build time via `prost-build`
- Hardcoded usage in `mcap_writer.rs`:
  ```rust
  let sensor_msg = crate::proto::SensorData {
      topic: topic.to_string(),
      timestamp_ns: timestamp as i64,
      frame_id: sample.key_expr.as_str().to_string(),
      payload: sample.payload.contiguous().to_vec(),
  };
  ```

## Problem Statement

Users want to:
1. **Provide their own proto definitions** (e.g., ROS messages, custom formats)
2. **Change proto schemas without recompiling** the recorder
3. **Store schema metadata** alongside data for proper deserialization

---

## Approach 1: Schema-Agnostic (Generic Binary) ✅ RECOMMENDED

### Concept
**Don't assume any proto schema** - just store raw binary payloads with optional schema metadata.

### Architecture

```rust
// New generic message wrapper
message RecordedMessage {
    string topic = 1;
    int64 timestamp_ns = 2;
    bytes payload = 3;  // Raw Zenoh payload (could be proto, JSON, etc.)
    
    // Schema metadata (optional)
    SchemaInfo schema = 4;
}

message SchemaInfo {
    string format = 1;      // "protobuf", "json", "msgpack", "flatbuffers"
    string schema_name = 2; // "sensor_msgs/Image", "my_msgs/CustomData"
    string schema_hash = 3; // MD5/SHA of schema for version tracking
    bytes schema_data = 4;  // Optional: embedded schema definition
}
```

### Implementation

```rust
// mcap_writer.rs - Generic implementation
pub struct GenericMcapWriter {
    // No proto assumptions
}

impl GenericMcapWriter {
    pub fn serialize_samples(
        &self,
        samples: &[Sample],
        topic: &str,
        recording_id: &str,
        schema_info: Option<SchemaInfo>,
    ) -> Result<Vec<u8>> {
        for sample in samples {
            // Just wrap raw bytes
            let msg = RecordedMessage {
                topic: topic.to_string(),
                timestamp_ns: get_timestamp(&sample),
                payload: sample.payload.contiguous().to_vec(),  // Raw bytes!
                schema: schema_info.clone(),
            };
            
            // Encode this minimal wrapper
            msg.encode(&mut buffer)?;
        }
        // ... compression and MCAP wrapping
    }
}
```

### Configuration

```yaml
recorder:
  # Schema configuration (optional)
  schema:
    format: protobuf  # or json, msgpack, raw
    schema_registry:
      enabled: false
      url: http://schema-registry:8081  # Optional Confluent-style registry
    
    # Per-topic schema hints
    per_topic:
      "/camera/image":
        format: protobuf
        schema_name: sensor_msgs.Image
      "/imu/data":
        format: json
```

### Pros
✅ **Most flexible** - works with any data format
✅ **No recompilation needed** - recorder is format-agnostic
✅ **Schema evolution** - easy to version schemas
✅ **Works with existing data** - users keep their serialization

### Cons
❌ **No built-in validation** - recorder doesn't validate payloads
❌ **Schema management** - users must handle schema registration
❌ **Requires external tools** - for schema-aware queries

### User Workflow

```bash
# 1. User serializes their data (e.g., with their own proto)
pub_data = MyCustomProto { ... }.encode()
zenoh.put("/my/topic", pub_data)

# 2. Recorder stores it as-is (no assumptions)
# Stores: { topic, timestamp, payload: <raw bytes>, schema: {...} }

# 3. User deserializes when reading
data = storage.get("/my/topic")
my_msg = MyCustomProto.decode(data.payload)
```

---

## Approach 2: Dynamic Proto Loading (Runtime) 🔄 COMPLEX

### Concept
Load `.proto` files at runtime and use reflection for dynamic encoding/decoding.

### Architecture

```rust
use prost_reflect::{DynamicMessage, DescriptorPool, MessageDescriptor};

pub struct DynamicProtoWriter {
    descriptor_pool: DescriptorPool,
    message_descriptors: HashMap<String, MessageDescriptor>,
}

impl DynamicProtoWriter {
    pub fn new(proto_files: &[PathBuf]) -> Result<Self> {
        // Parse .proto files at runtime
        let pool = DescriptorPool::decode(
            protox::compile(proto_files, &["."])?.as_slice()
        )?;
        
        Ok(Self {
            descriptor_pool: pool,
            message_descriptors: HashMap::new(),
        })
    }
    
    pub fn encode_message(
        &self,
        message_type: &str,
        data: &serde_json::Value,
    ) -> Result<Vec<u8>> {
        let descriptor = self.message_descriptors.get(message_type)?;
        let dynamic_msg = DynamicMessage::decode(descriptor, data)?;
        Ok(dynamic_msg.encode_to_vec())
    }
}
```

### Configuration

```yaml
recorder:
  proto_loader:
    enabled: true
    proto_paths:
      - /etc/protos/sensor_msgs.proto
      - /etc/protos/custom_msgs.proto
    include_dirs:
      - /usr/local/include
    
    # Topic to message type mapping
    topic_mappings:
      "/camera/image": "sensor_msgs.Image"
      "/imu/data": "sensor_msgs.Imu"
```

### Pros
✅ **Type-safe at runtime** - validates against schema
✅ **No recompilation** - load .proto files dynamically
✅ **Schema aware** - can introspect message structure

### Cons
❌ **Complex implementation** - requires `prost-reflect` + `protox`
❌ **Performance overhead** - slower than compiled protos
❌ **Limited ecosystem** - dynamic proto support is immature in Rust
❌ **Error handling** - harder to debug runtime schema errors

### Dependencies

```toml
[dependencies]
prost-reflect = "0.13"
protox = "0.6"
```

---

## Approach 3: Plugin System 🔌 FUTURE

### Concept
Users provide **serialization plugins** as dynamic libraries.

### Architecture

```rust
// Plugin trait
pub trait SerializerPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn serialize(&self, sample: &Sample) -> Result<Vec<u8>>;
    fn schema_info(&self) -> SchemaInfo;
}

// Plugin loader
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn SerializerPlugin>>,
}

impl PluginManager {
    pub fn load_plugin(&mut self, path: &Path) -> Result<()> {
        unsafe {
            let lib = libloading::Library::new(path)?;
            let constructor: libloading::Symbol<fn() -> Box<dyn SerializerPlugin>> =
                lib.get(b"create_plugin")?;
            let plugin = constructor();
            self.plugins.insert(plugin.name().to_string(), plugin);
        }
        Ok(())
    }
}
```

### Configuration

```yaml
recorder:
  plugins:
    - name: ros2_serializer
      path: /usr/lib/zenoh-recorder/plugins/libros2_serializer.so
      topics: ["/camera/**", "/lidar/**"]
    
    - name: json_serializer
      path: /usr/lib/zenoh-recorder/plugins/libjson_serializer.so
      topics: ["/telemetry/**"]
```

### Pros
✅ **Ultimate flexibility** - users write custom serializers
✅ **Performance** - compiled plugins, no overhead
✅ **Ecosystem friendly** - ROS, DDS, etc. can provide plugins

### Cons
❌ **Most complex** - requires plugin infrastructure
❌ **Safety concerns** - dynamic loading in Rust is unsafe
❌ **Maintenance burden** - plugin API versioning

---

## Approach 4: Build-Time Codegen (Current + Extended) 🔧

### Concept
Extend current approach: users add `.proto` files to `proto/` directory and rebuild.

### Architecture

```rust
// build.rs - Enhanced
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile all .proto files in proto/ and user-provided paths
    let mut proto_files = Vec::new();
    
    // Built-in protos
    proto_files.extend(glob::glob("proto/*.proto")?);
    
    // User protos (via env var)
    if let Ok(user_proto_path) = env::var("ZENOH_RECORDER_PROTO_PATH") {
        proto_files.extend(glob::glob(&format!("{}/*.proto", user_proto_path))?);
    }
    
    prost_build::Config::new()
        .out_dir("src/generated")
        .compile_protos(&proto_files, &["proto", &user_proto_path])?;
    
    Ok(())
}
```

### Configuration

Users specify which generated types to use:

```yaml
recorder:
  serialization:
    message_type: "my_custom_msgs::SensorData"  # Fully qualified type
```

### Pros
✅ **Performant** - compiled code, zero overhead
✅ **Type-safe** - compile-time checks
✅ **Simple** - extends existing build system

### Cons
❌ **Requires recompilation** - not runtime-flexible
❌ **Coupling** - recorder must know all types at build time
❌ **Distribution** - users need custom builds

---

## Recommendation: Hybrid Approach 🎯

Combine **Approach 1 (Schema-Agnostic)** with optional schema metadata:

### Phase 1: Generic Binary (Immediate)
- Store raw Zenoh payloads without assumptions
- Add optional `SchemaInfo` metadata
- Users handle serialization themselves

### Phase 2: Schema Registry Integration (v0.2)
- Add Confluent Schema Registry support
- Automatically attach schema metadata
- Enable schema evolution tracking

### Phase 3: Proto Reflection (v0.3)
- Add `prost-reflect` for runtime proto loading
- Optional feature: `--features dynamic-proto`

### Implementation Plan

```rust
// New configuration
pub struct SerializationConfig {
    pub mode: SerializationMode,
    pub schema_registry: Option<SchemaRegistryConfig>,
}

pub enum SerializationMode {
    Raw,           // Just store bytes (default)
    WithSchema,    // Store bytes + schema metadata
    DynamicProto,  // Runtime proto loading (future)
}

pub struct SchemaRegistryConfig {
    pub url: String,
    pub auth: Option<AuthConfig>,
}
```

### Migration Path

```rust
// Backward compatible
impl McapWriter {
    // Old API (deprecated)
    #[deprecated(since = "0.2.0", note = "Use serialize_samples_generic")]
    pub fn serialize_samples(
        &self,
        samples: &[Sample],
        topic: &str,
        recording_id: &str,
    ) -> Result<Vec<u8>> {
        // Use SensorData proto (old behavior)
        self.serialize_samples_generic(samples, topic, recording_id, None)
    }
    
    // New API
    pub fn serialize_samples_generic(
        &self,
        samples: &[Sample],
        topic: &str,
        recording_id: &str,
        schema_info: Option<SchemaInfo>,
    ) -> Result<Vec<u8>> {
        // Generic implementation
    }
}
```

---

## Questions to Consider

1. **What serialization formats do your users need?**
   - Just protobuf? JSON? Msgpack? Flatbuffers?

2. **Do you need runtime flexibility?**
   - Can users rebuild? Or must it be runtime-configurable?

3. **Schema evolution requirements?**
   - Do schemas change frequently?
   - Need backward compatibility?

4. **Performance vs Flexibility tradeoff?**
   - High-frequency recording needs compiled code
   - Low-frequency can use dynamic loading

5. **Integration with existing systems?**
   - ROS 2? DDS? Custom middleware?

---

## Example: Approach 1 Implementation

See next document: `CUSTOM_PROTO_IMPLEMENTATION.md`

