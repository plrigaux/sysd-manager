import sys

path = sys.argv[1]
content = sys.argv[2]

print(path)
print(content)

with open(path, "w") as f:
    f.write(content)
