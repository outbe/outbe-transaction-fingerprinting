# Transaction Fingerprinting Service

A distributed cryptographic service for generating privacy-preserving transaction fingerprints using collaborative computation and secret sharing techniques.

## Overview

This service implements a collaborative protocol for generating deterministic yet unpredictable transaction fingerprints. The system uses threshold secret sharing and multi-party computation to ensure that no single entity can generate or predict transaction fingerprints without cooperation from a minimum threshold of agents.

## Key Features

- **Privacy-Preserving**: Transaction fingerprints are generated without revealing sensitive transaction data
- **Distributed Trust**: Uses threshold secret sharing (e.g., 3-of-5) to prevent single points of failure
- **Cryptographic Security**: Built on BN256 elliptic curves and Poseidon hash functions
- **Scalable Architecture**: gRPC-based microservices with configurable agent topologies
- **Deterministic Output**: Same transaction data always produces the same fingerprint
- **Collision Resistant**: Extensive testing ensures no fingerprint collisions

## Architecture

### Core Components

#### Core Library
- **Transaction Fingerprinting**: Generate transaction fingerprints from transaction data
- **Secret Sharing**: Generate secret shares for threshold secret sharing
- **Poseidon Hash**: Poseidon hash function for generating fingerprints
- **Hash-to-Curve**: Elligator2 for mapping field elements to elliptic curve points
- **Compact Encoding**: Human-readable fingerprint representation

#### gRPC Services
- **Fingerprint Service**: Generate transaction fingerprints
- **Cooperation Service**: Internal communication between agents

#### CLI Tools
- **Agent Server**: Start agent servers for development
- **Light Agent Server**: Start light agent servers for production
- **CLI Utility**: Generate secret shares for threshold secret sharing

### Protocol Types

1. **Collaborative Protocol**: Multi-agent threshold secret sharing
2. **Naive Protocol**: Single-agent mode for testing/development

### Transaction Data Structure

Each transaction fingerprint is generated from:
- **BIC**: Bank Identifier Code (e.g., "BCEELU21")
- **Amount**: Transaction amount with precision (base + atto parts)
- **Currency**: ISO 4217 currency code
- **DateTime**: Transaction timestamp in UTC
- **WWD**: World Wide Day (associated date)

## Cryptographic Foundation

### Protocol Implementation

The collaborative protocol ensures that transaction fingerprints can only be generated through cooperation between 
multiple agents, preventing any single entity from controlling the fingerprint generation process.

**Key Protocol Properties:**
- **Threshold Security**: Requires t-out-of-n agents to cooperate (e.g., 3-of-5)
- **Blinding**: Uses random blinding factors to prevent information leakage
- **Lagrange Reconstruction**: Combines partial computations using Lagrange interpolation
- **BN256 Curve**: Elliptic curve for cryptographic operations

### Hash Functions
- **Poseidon Hash**: Primary hash function with configurable rounds (8 full rounds, 57 partial rounds)
- **Hash-to-Curve**: Elligator2 for mapping field elements to elliptic curve points
- **Base58 Encoding**: Compact representation of fingerprints for human readability

### Secret Sharing
- **Shamir's Secret Sharing**: Threshold-based secret distribution using polynomial interpolation
- **Lagrange Interpolation**: Secret reconstruction from shares with proper coefficients

### Implementation aspects

#### Curve 
- **Implementation**: BN256 curve
- **Rationale**: 
  - Wider adoption in existing cryptographic libraries
  - Optimized implementations available in Halo2 ecosystem
  - Better compatibility with blockchain and financial infrastructure
  - Maintains equivalent security guarantees

#### Hash Function Selection
- **Poseidon Hash**: Custom implementation with configurable rounds
- **Specifications**: 8 full rounds, 57 partial rounds for different input sizes
- **Domain Tag**: $T_s = 1 << 64$
- **Benefits**: 
  - Zero-knowledge friendly
  - Efficient in circuit-based computations
  - Resistant to known attacks

#### Additional Enhancements
- **Base58 Encoding**: Human-readable fingerprint representation
- **Parallel Processing**: Concurrent agent communication for improved performance
- **Comprehensive Testing**: Extensive test suite

### Protocol Workflow

The protocol follows a specific sequence to ensure secure, collaborative fingerprint generation:

#### Phase 1: Data Preparation
1. **Transaction Serialization**: Components (BIC, amount, currency, datetime) are serialized with a fixed 8-byte prefix
2. **Hash Computation**: Poseidon hash of serialized transaction data
3. **Hash-to-Curve**: Map hash to BN256 curve point using Elligator2

#### Phase 2: Collaborative Computation
4. **Blinding**: Initiating agent applies random blinding factor `r` to curve point
5. **Agent Coordination**: Request cooperation from threshold-1 other agents
6. **Partial Computation**: Each agent computes `[s_i] * [r] * P` where `s_i` is their secret share
7. **Response Collection**: Gather responses from cooperating agents

#### Phase 3: Reconstruction
8. **Lagrange Interpolation**: Combine partial results using Lagrange coefficients
9. **Unblinding**: Remove blinding factor to obtain `[k] * P`
10. **Final Hash**: Squeeze final fingerprint from the resulting curve point

#### Security Properties
- **Privacy**: No agent learns the original transaction data or final secret
- **Robustness**: Protocol succeeds as long as threshold number of agents cooperate
- **Unpredictability**: Fingerprints are cryptographically random and unpredictable
- **Determinism**: Same transaction data always produces identical fingerprint

## Installation & Setup

### Prerequisites
- Rust 1.90.0 or later
- Cargo package manager

### Build

```bash
# Build all components
cargo build --release

# Build specific binaries
cargo build --release --bin fingerprinting-agent
cargo build --release --bin fingerprinting-light-agent
cargo build --release --bin fingerprinting-cli
```

For building containers please use buildx. 

```bash
docker buildx build -t outbe/fingerprinting:1.0 --platform linux/amd64,linux/arm64 .
```

After building the docker image you can run the container with the following command:
```bash
 docker run -d -v $(pwd)/examples/docker/config:/config --name fingerprint -p 9000:9000 --platform=linux/amd64 outbe/fingerprinting:1.0
```

#### Building clients

Generation of the following clients are supported:
- go gRPC client
- kotlin gRPC client
- java gRPC client

Changes in protofiles are required to generate clients. 
They can be generated by the following command: 
```bash 
buf generate
```

The generated code will be placed in the `clients/{go|java|kotlin}` folders.

To build JVM Client artefacts, please use the following command: 
```bash
# in clients/java or clients/kotlin
mvn clean package
```

## Configuration

### Agent Configuration

Full Agents can run in two modes:

#### Cooperative Mode
```hocon
{
  # Public gRPC service endpoint
  grpc: {
    address: localhost
    port: 9000
  }
  # Internal agent-to-agent communication
  agent-grpc: {
    host: "[::]"
    port: 9001
  }

  # Agent configuration with static tology
  fingerprint-service: {
    type: Cooperative
    agent_id: 1
    secret_shard: "2q3CusLJFtX2r2Y42mkAtZGisPJ8BzyhkoTHgZ37WAF1"
    agents: 5
    threshold: 3
    members: [
      {agent_id: 2, address: "localhost:9002"},
      {agent_id: 3, address: "localhost:9003"},
      {agent_id: 4, address: "localhost:9004"},
      {agent_id: 5, address: "localhost:9005"}
    ]
  }
}
```

#### Naive Mode (Development)
```hocon
{
  # Public gRPC service endpoint
  grpc: {
    address: localhost
    port: 9000
  }
  # Internal agent-to-agent communication (ignored in development mode)
  agent-grpc: {
    host: "[::]"
    port: 9001
  }
  
  # Agent configuration for development purposes
  fingerprint-service: {
    type: Naive
    secret: "6hDkQUcrkMKWfjofiFAF3AAt4gBNEAtdyggeLxDVXyux"
  }
}
```

### Secret Sharing Setup

Generate secret shares for your agent network:

```bash
# Generate 3-of-5 secret sharing
./target/release/fingerprinting-cli --threshold 3 --agents 5
```

Output:
```
Random secret: 6hDkQUcrkMKWfjofiFAF3AAt4gBNEAtdyggeLxDVXyux
Shares:
== share 1: 2q3CusLJFtX2r2Y42mkAtZGisPJ8BzyhkoTHgZ37WAF1
== share 2: CBvxVKszXcLMVP5qTyB85zVx1FK71yQ9vgqDpuXhnXPi
== share 3: HUgDcACRCTdMtQ6iwgAe1VsXx2M6ChrDajJTVw7rc39e
== share 4: XugsmnUMuNCRio3LghH8YdJVFmtN3NzFCJxqoHvMfHs
== share 5: FugMM3q4yngpeCvZ7a6BqVXMGLVYLiBTLSygEdxJ2dg4
```

## Running the Service

### Development Mode (Single Agent)

```bash
./target/release/fingerprinting-agent --config examples/cra-fingerprint-config/agent-naive.conf
```

### Production Mode (Multi-Agent)

1. **Start Light Agents** (agents 2-5):
```bash
./target/release/fingerprinting-light-agent --config examples/t3s5-config/agent-2.conf
./target/release/fingerprinting-light-agent --config examples/t3s5-config/agent-3.conf
./target/release/fingerprinting-light-agent --config examples/t3s5-config/agent-4.conf
./target/release/fingerprinting-light-agent --config examples/t3s5-config/agent-5.conf
```

2. **Start Main Agent** (agent 1):
```bash
./target/release/fingerprinting-agent --config examples/cra-fingerprint-config/agent-1.conf
```

## Use Cases and Applications

This CRA-based transaction fingerprinting service is designed for **financial systems** and **regulatory compliance** scenarios where:

### Financial Services
- **Anti-Money Laundering (AML)**: Generate consistent transaction identifiers across institutions
- **Know Your Customer (KYC)**: Create privacy-preserving customer transaction profiles
- **Regulatory Reporting**: Generate standardized transaction fingerprints for compliance
- **Cross-Border Payments**: Coordinate transaction identification across jurisdictions

### Privacy-Preserving Analytics
- **Transaction Monitoring**: Detect patterns without revealing sensitive data
- **Risk Assessment**: Analyze transaction flows while maintaining privacy
- **Fraud Detection**: Identify suspicious patterns across multiple institutions
- **Audit Trails**: Create verifiable transaction records

### Key Requirements Addressed
- **Deterministic Output**: Same transaction always produces identical fingerprint
- **Unpredictable**: Fingerprints appear random and cannot be predicted
- **Distributed Trust**: No single entity controls fingerprint generation
- **Privacy Preservation**: Transaction data remains confidential
- **High Availability**: System continues operating with partial agent failures
- **Regulatory Compliance**: Meets financial industry security standards

## API Usage

### gRPC Services

#### Fingerprint Service
- **Endpoint**: `FingerprintService`
- **Method**: `GenerateFingerprint`
- **Input**: `TransactionFingerprintData`
- **Output**: `Fingerprint`

#### Cooperation Service
- **Endpoint**: `CooperationService`
- **Method**: `ComputeExponent`
- **Purpose**: Internal agent-to-agent communication

### Example Transaction Data

```protobuf
message TransactionFingerprintData {
  string bic = 1;                    // "BCEELU21"
  Money amount = 10;                 // {amount_base: 1000, amount_atto: 0, currency: "EUR"}
  Timestamp date_time = 20;          // UTC timestamp
  Date wwd = 30;                     // World Wide Day
}
```

## Mathematical Foundations

### Given

$q = \text{big prime}$; $G$: prime-order group over $q$; $\chi: \{0, 1\}^* \to G$

$H: \{0, 1\}^* \to \{0, 1\}^* $ PoseidonHash(Fq, 8, 57)

$k \leftarrow F_q$

### Setup correctness

By Shamir's Secret Sharing, polynomial $P(x) = k + a_1 x + a_2 x^2 + \ldots + a_{t-1} x^{t-1}$ ensures:
- $P(0) = k$ (the secret)
- Any $t$ points $(i, P(i))$ uniquely determine $P(x)$
- Any $t - 1$ or fewer points reveal no information about $k$ (information-theoretic security)

### Blinding correctness

The requester computes $B = [r] P$ where $P = \chi(H(T_s | WWD | Nonce))$.

By properties of scalar multiplication: $B \in G$ and $r$ is known only to the requester.

### Distributed Evaluation correctness

Each cooperating agent $i$ computes: $E_i = [s_i] B$ which never reveals the secret shard $s_i$ to requester not gives information about $P$ to the agent.

### Lagrange Interpolation in the Exponent correctness

Compute Lagrange coefficients for interpolation at x = 0

$$\lambda_i = \prod_{j \in S, j \neq i} \frac{0-j}{i-j} = \prod_{j \in S, j \neq i} \frac{-j}{i-j} \mod q$$

By Lagrange interpolation property (S - set of cooperative agents of t size):

$$\sum_{i \in S} \lambda_i \cdot s_i = \sum_{i \in S} \lambda_i \cdot P(i) = P(0) = k$$

Combine evaluations using group law:

$$Y = \sum_{i \in S} [\lambda_i] E_i = \sum_{i \in S} [\lambda_i]([s_i \cdot r] P) = \sum_{i \in S} [\lambda_i \cdot s_i \cdot r]P = \left[r \sum_{i \in S} \lambda_i \cdot s_i\right] P = [r \cdot k] P$$

### Unblinding correctness:

$$Y = [r^{-1}] Y = [r^{-1}]([r \cdot k] P) = [r^{-1} \cdot r \cdot k] P = [k] P$$

## Security Considerations

### Protocol Security

The protocol provides several security guarantees:

**Threshold Security:**
- Requires cooperation of at least `t` agents to generate fingerprints
- Any subset of `t-1` or fewer agents cannot compute valid fingerprints
- Secret shares are information-theoretically secure

**Privacy Preservation:**
- Transaction data is never revealed to individual agents
- Blinding factors prevent information leakage during computation
- Final fingerprint reveals no information about input data

**Unpredictability:**
- Fingerprints are cryptographically random and unpredictable
- No agent can predict fingerprints without cooperation
- Generation numbers prevent replay attacks

### Threat Model
- **Single Agent Compromise**: Insufficient to generate fingerprints
- **Threshold Attack**: Requires compromising multiple agents simultaneously
- **Network Attacks**: gRPC communication should be secured in production
- **Replay Attacks**: Generation numbers prevent replay of old computations
- **Side-Channel Attacks**: Constant-time implementations prevent timing attacks

### Best Practices
1. **Network Security**: Use TLS for gRPC communication
2. **Agent Isolation**: Deploy agents on separate infrastructure and under different legal entity control
3. **Secret Management**: Secure storage of secret shares using HSMs
4. **Monitoring**: Implement health checks and telemetry
5. **Access Control**: Authenticate agent-to-agent communication
6. **Regular Rotation**: Periodically rotate secret shares
7. **Audit Logging**: Log all fingerprint generation requests

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
The test suite includes:
- Secret sharing reconstruction tests
- Multi-agent protocol tests
- Cryptographic primitive tests

## Development

### Project Structure

```
transaction-fingerprinting/
├── crates/
│   ├── fingerprinting-core/          # Core fingerprinting logic
│   ├── fingerprinting-cli/           # CLI tools and agent servers
│   ├── fingerprinting-grpc/          # gRPC service definitions
│   ├── fingerprinting-grpc-agent/    # Agent cooperation protocol
│   ├── fingerprinting-poseidon/      # Poseidon hash implementation (Based on https://github.com/axiom-crypto/pse-poseidon repo) 
│   └── fingerprinting-types/         # Common type definitions
├── examples/                         # Configuration examples
└── Cargo.toml                        # Workspace configuration
```

### Adding New Features

1. **New Collaborative Protocols**: Implement `FingerprintProtocol` trait in `fingerprinting-core` crate
2. **New Hash Components**: Add to `components/` module  in `fingerprinting-core` crate
3. **New Services**: Define gRPC protobuf files in `fingerprinting-grpc` crate
4. **Configuration**: Extend configuration structures 

## Roadmap

### Planned Features
- [ ] Blockchain-based topology discovery
- [ ] Agent authentication and authorization
- [x] Docker containerization with musl
- [ ] Health checks and telemetry reporting
- [x] Multi-language SDKs (Java, Kotlin, Go)
- [ ] CLI client for fingerprint generation
- [ ] Rate limiting for unauthenticated agents

### Performance Optimizations
- [x] Parallel agent communication
- [x] Connection pooling

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see the LICENSE file for details.

---

**Note**: This service is designed for production use in financial systems. Ensure proper security auditing and testing before deployment in critical environments.
