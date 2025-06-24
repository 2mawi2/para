# Crystal Hello World

A comprehensive Crystal Hello World implementation demonstrating Crystal language features, testing, and project structure.

## Features

- **Multiple Greeting Methods**: Basic and personalized greeting functionality
- **String Operations**: Uppercase, lowercase, and validation methods
- **Comprehensive Testing**: Full test suite using Crystal's built-in spec framework
- **Type Safety**: Leverages Crystal's compile-time type checking
- **Clean Architecture**: Well-organized module structure with proper separation of concerns

## Project Structure

```
crystal-hello/
├── shard.yml              # Project configuration and dependencies
├── src/
│   ├── hello.cr           # Main Hello module with greeting methods
│   └── main.cr            # Application entry point
├── spec/
│   └── hello_spec.cr      # Comprehensive test suite
└── README.md              # This file
```

## Installation

### Prerequisites

- Crystal (>= 1.0.0)
- Shards (Crystal's package manager)

### Setup

1. Navigate to the project directory:
   ```bash
   cd crystal-hello
   ```

2. Install dependencies:
   ```bash
   shards install
   ```

## Usage

### Running the Application

```bash
# Run directly with Crystal
crystal run src/main.cr

# Or compile and run
crystal build src/main.cr -o hello
./hello
```

### Expected Output

```
Hello, World!
Hello, Crystal!
Hello, World!
Greeting length: 13
Uppercase: HELLO, WORLD!
Lowercase: hello, world!
"Hello, World!" is valid
"Hi there!" is invalid
"Hello, Crystal!" is valid
```

## API Reference

### Hello Module

The `Hello` module provides various greeting-related methods:

#### Basic Greetings

- `Hello.get_greeting() : String` - Returns "Hello, World!"
- `Hello.get_greeting(name : String) : String` - Returns personalized greeting
- `Hello.print_greeting() : Nil` - Prints basic greeting to stdout
- `Hello.print_greeting(name : String) : Nil` - Prints personalized greeting

#### String Operations

- `Hello.get_greeting_upcase() : String` - Returns greeting in uppercase
- `Hello.get_greeting_downcase() : String` - Returns greeting in lowercase
- `Hello.greeting_length() : Int32` - Returns length of the greeting

#### Validation and Utilities

- `Hello.valid_greeting?(greeting : String) : Bool` - Validates greeting format
- `Hello.get_custom_greeting(message : String) : String` - Returns custom message

### Usage Examples

```crystal
require "./src/hello"

# Basic usage
puts Hello.get_greeting                    # "Hello, World!"
puts Hello.get_greeting("Crystal")         # "Hello, Crystal!"

# String operations
puts Hello.get_greeting_upcase             # "HELLO, WORLD!"
puts Hello.greeting_length                 # 13

# Validation
puts Hello.valid_greeting?("Hello, World!")  # true
puts Hello.valid_greeting?("Hi there!")      # false
```

## Testing

### Running Tests

```bash
# Run all tests
crystal spec

# Run tests with verbose output
crystal spec --verbose

# Run specific test file
crystal spec spec/hello_spec.cr
```

### Test Coverage

The test suite covers:

- **Basic Functionality**: All greeting methods return correct values
- **Type Safety**: Ensures methods return expected types
- **Edge Cases**: Empty strings, whitespace, special characters
- **String Operations**: Uppercase, lowercase, validation
- **Input Validation**: Various greeting formats and edge cases
- **Unicode Support**: Names with special characters
- **Output Testing**: Verifies stdout output for print methods

### Test Structure

```crystal
describe Hello do
  describe ".get_greeting" do
    it "returns the correct greeting message" do
      Hello.get_greeting.should eq("Hello, World!")
    end
    # ... more tests
  end
  # ... more describe blocks
end
```

## Crystal Language Features Demonstrated

1. **Modules**: Organizing code in the `Hello` module
2. **Method Overloading**: Multiple `get_greeting` methods with different signatures
3. **Type Annotations**: Explicit return types (`: String`, `: Int32`, `: Bool`)
4. **String Interpolation**: Using `#{}` syntax for dynamic strings
5. **Method Chaining**: String methods like `.strip.empty?`
6. **Compile-time Checks**: Type safety enforced at compile time
7. **Spec Framework**: Built-in testing with descriptive specs
8. **Pattern Matching**: String validation with starts_with/ends_with
9. **Memory Safety**: Automatic memory management
10. **Performance**: Compiled to native code for fast execution

## Building and Distribution

### Development Build

```bash
crystal build src/main.cr -o hello-dev
```

### Release Build

```bash
crystal build src/main.cr -o hello --release
```

### Cross-platform Considerations

Crystal compiles to native binaries for the target platform. The code is written to be cross-platform compatible.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `crystal spec`
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Learn More

- [Crystal Language Documentation](https://crystal-lang.org/docs/)
- [Crystal Spec Framework](https://crystal-lang.org/api/latest/Spec.html)
- [Crystal Shards](https://crystal-lang.org/reference/the_shards_command/)