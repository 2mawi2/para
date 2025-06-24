require "./hello"

# Main entry point for the Crystal Hello World application
module Main
  def self.run
    # Print the basic greeting
    Hello.print_greeting

    # Demonstrate personalized greetings
    Hello.print_greeting("Crystal")
    Hello.print_greeting("World")
    
    # Show some additional functionality
    puts "Greeting length: #{Hello.greeting_length}"
    puts "Uppercase: #{Hello.get_greeting_upcase}"
    puts "Lowercase: #{Hello.get_greeting_downcase}"
    
    # Demonstrate validation
    sample_greetings = ["Hello, World!", "Hi there!", "Hello, Crystal!"]
    sample_greetings.each do |greeting|
      valid = Hello.valid_greeting?(greeting)
      puts "\"#{greeting}\" is #{valid ? "valid" : "invalid"}"
    end
  end
end

# Run the application if this file is executed directly
if PROGRAM_NAME == __FILE__
  Main.run
end