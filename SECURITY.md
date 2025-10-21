# Security Policy

## Reporting Security Issues

If you discover a security vulnerability in zenoh-recorder, please email us at security@coscene.io. Please do not file public issues for security vulnerabilities.

## Known Security Advisories

### Transitive Dependencies from Zenoh

The following advisories are from transitive dependencies of `zenoh = "0.11"` and will be resolved when Zenoh releases updates:

#### 1. ring 0.16.20 (RUSTSEC-2025-0009, RUSTSEC-2025-0010)

**Status:** ⚠️ Monitoring

**Impact:** 
- **RUSTSEC-2025-0009**: AES functions may panic when overflow checking is enabled
  - Only affects debug builds or when `overflow-checks = true` is explicitly set
  - Release builds are NOT affected
  - QUIC protocol could be affected by specially-crafted packets
  
- **RUSTSEC-2025-0010**: ring 0.16.x is unmaintained
  - Latest maintained version is 0.17.12+

**Mitigation:**
- We build release binaries without overflow checking (standard practice)
- Monitoring Zenoh project for updates to ring 0.17+
- Risk is LOW for production deployments

#### 2. RSA 0.9.8 (RUSTSEC-2023-0071)

**Status:** ⚠️ Monitoring

**Impact:** Marvin Attack - potential key recovery through timing sidechannels

**Mitigation:**
- zenoh-recorder does not directly use RSA encryption
- This is a transitive dependency only
- No RSA keys are generated or used in our codebase
- Risk is NONE for zenoh-recorder functionality

#### 3. async-std (RUSTSEC-2025-0052)

**Status:** ✅ Acceptable

**Impact:** Crate is discontinued (informational, not a vulnerability)

**Mitigation:**
- Used by Zenoh internally
- Still functional, no actual security vulnerability
- Alternative (smol) suggested by advisory
- Waiting for Zenoh to migrate

#### 4. instant (RUSTSEC-2024-0384)

**Status:** ✅ Acceptable  

**Impact:** Crate is unmaintained (informational, not a vulnerability)

**Mitigation:**
- Transitive dependency
- Still functional
- No security impact

#### 5. paste (RUSTSEC-2024-0436)

**Status:** ✅ Acceptable

**Impact:** Crate is unmaintained (informational, not a vulnerability)

**Mitigation:**
- Used by proc-macros at compile time only
- No runtime security impact
- Fork (pastey) available if needed

## Security Best Practices

When deploying zenoh-recorder:

1. **Always use release builds** for production
2. **Enable TLS** when connecting to ReductStore over networks
3. **Use environment variables** for sensitive configuration (API tokens)
4. **Restrict network access** to Zenoh and ReductStore endpoints
5. **Monitor logs** for unusual activity
6. **Keep dependencies updated** by regularly running `cargo update`

## Audit Configuration

We use `cargo-audit` with the following configuration (see `audit.toml`):

- Transitive dependency advisories are **warned** but don't fail CI
- Only HIGH and CRITICAL vulnerabilities in direct dependencies fail CI
- Unmaintained crates are tracked but don't block releases

## Update Schedule

- Dependencies are reviewed monthly
- Security patches are applied within 48 hours
- Major version updates are evaluated quarterly

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

---

Last updated: 2025-10-21

