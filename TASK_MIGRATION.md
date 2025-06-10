```
z "$CONFIG_FILE" ]; then
     23      if [ "$(uname)" = "Darwin" ]; then
     24          CONFIG_FILE="$HOME/Library/Application Support/para/config.json"
     24          CONFIG_FILE="$HOME/Library/Application 
     Support/para-rs/config.json"
        n"
     25      else
     26          CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/para/config.json"
     26          
     CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/para-rs/config.json"
        n"
     27      fi
     28  fi
```

remove the rust reference of its config file (where it still uses para-rsl) and change it to 'para'
when replacing the legacy implementation with the new one

```