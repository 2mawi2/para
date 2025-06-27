import os

home_dir = os.path.expanduser("~")
file_path = os.path.join(home_dir, "hello.txt")

with open(file_path, "w") as f:
    f.write("hello world")

print(f"File 'hello.txt' created in {home_dir}")
