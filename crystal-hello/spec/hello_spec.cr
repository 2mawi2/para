require "spec"
require "../src/hello"

describe Hello do
  describe ".get_greeting" do
    it "returns the correct greeting message" do
      Hello.get_greeting.should eq("Hello, World!")
    end

    it "returns the correct type" do
      Hello.get_greeting.should be_a(String)
    end

    it "returns personalized greeting with name" do
      Hello.get_greeting("Crystal").should eq("Hello, Crystal!")
    end

    it "handles empty name gracefully" do
      Hello.get_greeting("").should eq("Hello, World!")
    end

    it "handles whitespace-only name" do
      Hello.get_greeting("   ").should eq("Hello, World!")
    end

    it "trims whitespace from names" do
      Hello.get_greeting("  Crystal  ").should eq("Hello, Crystal!")
    end

    it "works with special characters in names" do
      Hello.get_greeting("Crystal 1.0").should eq("Hello, Crystal 1.0!")
    end
  end

  describe ".print_greeting" do
    it "prints the greeting to stdout" do
      output = capture_output { Hello.print_greeting }
      output.should eq("Hello, World!\n")
    end

    it "prints personalized greeting to stdout" do
      output = capture_output { Hello.print_greeting("Test") }
      output.should eq("Hello, Test!\n")
    end
  end

  describe ".get_custom_greeting" do
    it "returns custom message when provided" do
      Hello.get_custom_greeting("Welcome!").should eq("Welcome!")
    end

    it "returns default greeting for empty message" do
      Hello.get_custom_greeting("").should eq("Hello, World!")
    end

    it "trims whitespace from custom message" do
      Hello.get_custom_greeting("  Welcome!  ").should eq("Welcome!")
    end
  end

  describe ".valid_greeting?" do
    it "validates correct Hello World greeting" do
      Hello.valid_greeting?("Hello, World!").should be_true
    end

    it "validates personalized greetings" do
      Hello.valid_greeting?("Hello, Crystal!").should be_true
    end

    it "rejects invalid greetings" do
      Hello.valid_greeting?("Hi there!").should be_false
    end

    it "rejects greetings without exclamation" do
      Hello.valid_greeting?("Hello, World").should be_false
    end

    it "rejects greetings with wrong format" do
      Hello.valid_greeting?("Goodbye, World!").should be_false
    end

    it "handles empty strings" do
      Hello.valid_greeting?("").should be_false
    end
  end

  describe ".greeting_length" do
    it "returns correct length of greeting" do
      Hello.greeting_length.should eq(13)
    end

    it "returns integer type" do
      Hello.greeting_length.should be_a(Int32)
    end
  end

  describe ".get_greeting_upcase" do
    it "returns greeting in uppercase" do
      Hello.get_greeting_upcase.should eq("HELLO, WORLD!")
    end
  end

  describe ".get_greeting_downcase" do
    it "returns greeting in lowercase" do
      Hello.get_greeting_downcase.should eq("hello, world!")
    end
  end

  describe "string operations" do
    it "greeting contains comma" do
      Hello.get_greeting.includes?(",").should be_true
    end

    it "greeting starts with Hello" do
      Hello.get_greeting.starts_with?("Hello").should be_true
    end

    it "greeting ends with exclamation" do
      Hello.get_greeting.ends_with?("!").should be_true
    end
  end

  describe "edge cases" do
    it "handles very long names" do
      long_name = "a" * 100
      result = Hello.get_greeting(long_name)
      result.should start_with("Hello, ")
      result.should end_with("!")
      result.should contain(long_name)
    end

    it "handles names with special unicode characters" do
      Hello.get_greeting("José").should eq("Hello, José!")
    end

    it "handles names with numbers" do
      Hello.get_greeting("User123").should eq("Hello, User123!")
    end
  end
end

# Helper method to capture stdout output
private def capture_output(&block)
  old_stdout = STDOUT
  io = IO::Memory.new
  STDOUT = io
  yield
  io.to_s
ensure
  STDOUT = old_stdout
end