# Documentation Index

Welcome to redis-oxide documentation! This index helps you find what you need.

## 🚀 Quick Links

- **New to redis-oxide?** → Start with [Getting Started](GETTING_STARTED.md)
- **Already familiar?** → Check [Advanced Usage](README.md#advanced-features)
- **Having issues?** → See [Troubleshooting](TROUBLESHOOTING.md)
- **Want to contribute?** → Read [Contributing](CONTRIBUTING.md)

## 📚 Documentation Structure

### For Users

#### [README.md](README.md) - Complete Feature Overview
Main documentation covering:
- ✅ Installation and quick start
- ✅ Core operations (strings, hashes, lists, sets, sorted sets)
- ✅ Advanced features (pipelines, transactions, Lua scripting, Pub/Sub, streams)
- ✅ Connection strategies (multiplexed vs pooled)
- ✅ Cluster and Sentinel support
- ✅ Configuration options
- ✅ Error handling

**Read this if you want:** A comprehensive overview of all features

#### [Getting Started](GETTING_STARTED.md) - Beginner's Guide
Step-by-step introduction covering:
- 📦 Installation and setup
- 🚀 Your first program
- 📝 Common patterns and examples
- 🔧 Configuration options
- 🧪 Testing your connection
- ❓ Troubleshooting common issues

**Read this if you want:** To get up and running quickly

#### [Troubleshooting](TROUBLESHOOTING.md) - Problem Solving
Detailed solutions for common issues:
- 🔌 Connection issues
- 🔐 Authentication problems
- 🎯 Cluster issues
- 📊 Performance problems
- 💾 Memory issues
- 🔄 Pipeline/Transaction issues
- 📢 Pub/Sub problems
- 🛡️ Sentinel issues
- 🐛 Debugging tips

**Read this if you want:** To solve a specific problem

#### [Performance](PERFORMANCE.md) - Optimization Guide
Performance optimization and best practices:
- ⚡ Performance characteristics
- 🎯 Optimization techniques
- 📊 Throughput comparison
- 💡 Data structure selection
- 🔑 Key design strategies
- 📈 Benchmarking and profiling
- 🚨 Common performance issues

**Read this if you want:** To optimize your application

#### [Architecture](ARCHITECTURE.md) - Technical Deep Dive
Internal architecture and design decisions:
- 🏗️ Core components
- 📊 Data flow diagrams
- 🔄 Concurrency model
- 📁 Module structure
- 🎯 Design decisions
- 🧪 Testing strategy

**Read this if you want:** To understand how redis-oxide works internally

### For Contributors

#### [Contributing](CONTRIBUTING.md) - Contribution Guide
Guidelines for contributing:
- 👨‍💻 Development setup
- 📝 Code guidelines
- 🧪 Testing requirements
- 🔄 Pull request process
- 📚 Documentation standards

**Read this if you want:** To contribute to the project

#### [Roadmap](ROADMAP.md) - Future Plans
Development roadmap:
- ✅ Current features (v0.2.2)
- 🚀 Planned features
- 📅 Timeline

**Read this if you want:** To know what's coming next

### API Documentation

- **Official API Docs**: https://docs.rs/redis-oxide

## 🎯 Find What You Need

### By Topic

#### Connection & Setup
- [README: Installation](README.md#-installation)
- [Getting Started: Setup](GETTING_STARTED.md#project-setup)
- [Troubleshooting: Connection Issues](TROUBLESHOOTING.md#connection-issues)

#### String Operations
- [README: String Operations](README.md#string-operations)
- [Getting Started: Multiple Operations](GETTING_STARTED.md#handling-multiple-operations)

#### Collections (Hash, List, Set, Sorted Set)
- [README: Hash Operations](README.md#hash-operations)
- [README: List Operations](README.md#list-operations)
- [README: Set Operations](README.md#set-operations)
- [README: Sorted Set Operations](README.md#sorted-set-operations)
- [Getting Started: Working with Collections](GETTING_STARTED.md#working-with-collections)
- [Performance: Data Structure Selection](PERFORMANCE.md#3-appropriate-data-structures)

#### Advanced Features
- [README: Pipelines](README.md#pipeline-operations)
- [README: Transactions](README.md#transactions)
- [README: Lua Scripting](README.md#lua-scripting)
- [README: Pub/Sub](README.md#pubsub-messaging)
- [README: Redis Streams](README.md#redis-streams)

#### Cluster & High Availability
- [README: Redis Cluster](README.md#cluster-support)
- [README: Sentinel](README.md#high-availability-with-sentinel)
- [Troubleshooting: Cluster Issues](TROUBLESHOOTING.md#cluster-issues)

#### Performance
- [README: Configuration](README.md#-configuration)
- [Performance: Optimization Techniques](PERFORMANCE.md#optimization-techniques)
- [Performance: Benchmarks](PERFORMANCE.md#performance-benchmarks)
- [Troubleshooting: Performance Issues](TROUBLESHOOTING.md#performance-issues)

#### Error Handling
- [README: Error Handling](README.md#-error-handling)
- [Getting Started: Error Handling](GETTING_STARTED.md#error-handling)
- [Troubleshooting: Various Issues](TROUBLESHOOTING.md)

#### Development
- [Contributing: Contributing Guidelines](CONTRIBUTING.md)
- [Getting Started: Testing](GETTING_STARTED.md#testing-your-connection)
- [Architecture: Development Strategy](ARCHITECTURE.md#testing-strategy)

### By Experience Level

#### Beginner
Start here in order:
1. 📖 [README: Quick Start](README.md#-quick-start)
2. 📖 [Getting Started](GETTING_STARTED.md)
3. 📖 [README: Core Operations](README.md#-core-operations)
4. 💡 Try examples from [examples/](examples/)

#### Intermediate
Then explore:
1. 📖 [README: Advanced Features](README.md#-advanced-features)
2. 📖 [Performance: Optimization](PERFORMANCE.md)
3. 📖 [README: Configuration](README.md#-configuration)
4. ❓ [Troubleshooting](TROUBLESHOOTING.md)

#### Advanced
Deep dive into:
1. 📖 [Architecture](ARCHITECTURE.md)
2. 📖 [Performance: Benchmarks](PERFORMANCE.md#performance-benchmarks)
3. 💻 [Source Code](redis-oxide/src/)
4. 🔄 [Contributing](CONTRIBUTING.md)

## 📋 Learning Path

### Path 1: Basic Redis Operations (30 minutes)
1. Read [Getting Started: Prerequisites](GETTING_STARTED.md#prerequisites)
2. Follow [Getting Started: Project Setup](GETTING_STARTED.md#project-setup)
3. Try [Getting Started: Common Patterns](GETTING_STARTED.md#common-patterns)

### Path 2: Production Ready (2 hours)
1. Complete Path 1
2. Read [Performance](PERFORMANCE.md)
3. Read [README: Configuration](README.md#-configuration)
4. Read [Troubleshooting: Connection Issues](TROUBLESHOOTING.md#connection-issues)

### Path 3: Advanced Features (3 hours)
1. Complete Path 2
2. Read [README: Advanced Features](README.md#-advanced-features)
3. Try examples in [examples/](examples/)
4. Explore [Troubleshooting](TROUBLESHOOTING.md) for specific topics

### Path 4: Contributing (4 hours)
1. Complete Path 3
2. Read [Contributing](CONTRIBUTING.md)
3. Read [Architecture](ARCHITECTURE.md)
4. Explore source code in [redis-oxide/src/](redis-oxide/src/)

## 🔍 Common Questions

### "How do I get started?"
→ Read [Getting Started](GETTING_STARTED.md)

### "How do I connect to Redis?"
→ See [README: Quick Start](README.md#-quick-start)

### "What connection strategy should I use?"
→ Read [Performance: Connection Strategy Selection](PERFORMANCE.md#1-connection-strategy-selection)

### "How do I handle errors?"
→ See [README: Error Handling](README.md#-error-handling) or [Getting Started: Error Handling](GETTING_STARTED.md#error-handling)

### "Why is my application slow?"
→ Check [Performance: Common Performance Issues](PERFORMANCE.md#common-performance-issues)

### "How do I use Redis Cluster?"
→ Read [README: Cluster Support](README.md#cluster-support)

### "How do I set up Sentinel?"
→ See [README: High Availability with Sentinel](README.md#high-availability-with-sentinel)

### "What data structure should I use?"
→ Read [Performance: Appropriate Data Structures](PERFORMANCE.md#3-appropriate-data-structures)

### "How do I contribute?"
→ Read [Contributing](CONTRIBUTING.md)

### "What's the roadmap?"
→ See [Roadmap](ROADMAP.md)

## 📚 External Resources

### Official Redis Documentation
- [Redis Command Reference](https://redis.io/commands)
- [Redis Cluster Specification](https://redis.io/topics/cluster-spec)
- [Redis Sentinel Documentation](https://redis.io/topics/sentinel)
- [Redis Pub/Sub](https://redis.io/topics/pubsub)
- [Redis Streams](https://redis.io/topics/streams)

### Performance & Optimization
- [Redis Optimization](https://redis.io/topics/optimization)
- [Redis Protocol Specification](https://redis.io/topics/protocol-spec)

### Rust & Async
- [Tokio Documentation](https://tokio.rs)
- [Async Rust](https://rust-lang.github.io/async-book/)

## 📝 File Structure

```
redis-oxide/
├── README.md              ← Main documentation
├── GETTING_STARTED.md     ← Beginner's guide
├── TROUBLESHOOTING.md     ← Problem solving
├── PERFORMANCE.md         ← Performance guide
├── ARCHITECTURE.md        ← Technical deep dive
├── CONTRIBUTING.md        ← Contribution guide
├── ROADMAP.md             ← Future plans
├── INDEX.md               ← This file
├── examples/              ← Code examples
├── redis-oxide/
│   ├── src/               ← Source code
│   ├── tests/             ← Tests
│   └── benches/           ← Benchmarks
└── .github/workflows/     ← CI/CD
```

## 🆘 Getting Help

### Before Asking for Help

1. Check [Troubleshooting](TROUBLESHOOTING.md) for your issue
2. Search [GitHub Issues](https://github.com/nghiaphamln/redis-oxide/issues)
3. Review [README](README.md) and relevant docs
4. Check [examples/](examples/) for similar use cases

### Where to Get Help

- **Documentation**: This index and the files listed
- **Examples**: [examples/](examples/) directory
- **Issues**: [GitHub Issues](https://github.com/nghiaphamln/redis-oxide/issues)
- **Discussions**: [GitHub Discussions](https://github.com/nghiaphamln/redis-oxide/discussions)
- **API Docs**: [docs.rs/redis-oxide](https://docs.rs/redis-oxide)

## 📞 Support

- 🐛 **Bug Report**: [GitHub Issues](https://github.com/nghiaphamln/redis-oxide/issues)
- 💡 **Feature Request**: [GitHub Issues](https://github.com/nghiaphamln/redis-oxide/issues)
- 🤔 **Question**: [GitHub Discussions](https://github.com/nghiaphamln/redis-oxide/discussions)
- 📖 **Documentation**: File an issue or PR to improve docs

---

**Last Updated**: October 2025
**Version**: redis-oxide 0.2.2
