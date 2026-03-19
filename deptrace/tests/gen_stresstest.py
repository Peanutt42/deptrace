# stack overflow for values >10000, but thats totally acceptable and having a chain of >10000 libraries is far from practical
N=1000

with open("stresstest.deptrace.toml", "w") as f:
    f.write(
"""name = \"subdeps_resolving_stresstest\"

[dependencies]
""")
    for i in range(N):
        f.write(f"foo{i} = {{ kind = \"Runtime\", subdependencies = [\"foo{i+1}\"] }}\n")
    f.write(f"foo{N} = {{ kind = \"Runtime\", subdependencies = [] }}")
