// Copyright 2025 coScene
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use zenoh_recorder::storage::topic_to_entry_name;

#[test]
fn test_topic_to_entry_name() {
    assert_eq!(topic_to_entry_name("/camera/front"), "camera_front");
    assert_eq!(topic_to_entry_name("/lidar/points"), "lidar_points");
    assert_eq!(topic_to_entry_name("/imu/data"), "imu_data");
    assert_eq!(topic_to_entry_name("camera/front"), "camera_front");
    assert_eq!(topic_to_entry_name("/a/b/c/d"), "a_b_c_d");
    assert_eq!(topic_to_entry_name("/test/**"), "test_all");
}

#[test]
fn test_topic_to_entry_name_special_chars() {
    assert_eq!(topic_to_entry_name("/topic-with-dash"), "topic-with-dash");
    assert_eq!(
        topic_to_entry_name("/topic_with_underscore"),
        "topic_with_underscore"
    );
}
