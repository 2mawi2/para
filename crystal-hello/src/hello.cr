module Hello
  # Returns the classic greeting message
  def self.get_greeting : String
    "Hello, World!"
  end

  # Returns a personalized greeting message
  def self.get_greeting(name : String) : String
    if name.strip.empty?
      get_greeting
    else
      "Hello, #{name.strip}!"
    end
  end

  # Prints the greeting message to stdout
  def self.print_greeting : Nil
    puts get_greeting
  end

  # Prints a personalized greeting message to stdout
  def self.print_greeting(name : String) : Nil
    puts get_greeting(name)
  end

  # Returns greeting with custom message
  def self.get_custom_greeting(message : String) : String
    if message.strip.empty?
      get_greeting
    else
      message.strip
    end
  end

  # Validates if a greeting is the expected Hello World format
  def self.valid_greeting?(greeting : String) : Bool
    greeting == "Hello, World!" || greeting.starts_with?("Hello, ") && greeting.ends_with?("!")
  end

  # Returns the length of the greeting
  def self.greeting_length : Int32
    get_greeting.size
  end

  # Returns the greeting in uppercase
  def self.get_greeting_upcase : String
    get_greeting.upcase
  end

  # Returns the greeting in lowercase
  def self.get_greeting_downcase : String
    get_greeting.downcase
  end
end