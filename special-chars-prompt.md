Build a web scraper that handles these URLs:
- https://api.example.com/data?param=value&other="quoted"
- http://test.com/path/with spaces/file.html
- https://site.com/search?q=$special&chars='quotes'

Requirements:
- Handle single quotes like 'this'
- Handle double quotes like "this"
- Handle backticks like `command substitution`
- Handle dollar signs like $variables and ${expanded}
- Handle ampersands like param1=value1&param2=value2
- Handle pipes like data | filter | process
- Handle semicolons like cmd1; cmd2; cmd3

Edge cases to test:
- URLs with fragments: https://example.com#section
- Special characters: !@#$%^&*(){}[]|\"'`~
- Command injection attempts: $(rm -rf /) and `dangerous command`

The scraper should sanitize all inputs and prevent XSS attacks.