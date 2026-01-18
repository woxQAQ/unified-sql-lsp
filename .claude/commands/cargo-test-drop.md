# Cargo Test Drop

Drop all the Cargo test in the codebase.

# What's a Cargo test?

Cargo test, or called trivial test, is something that caused by the laziness engineers
with the idea that "we have coverage requirement, so let's add some test codes" or 
"the others writes tests codes, so I need to write tests too", they write
test just to let the codes to have tests. 

The Cargo test are usually numerous, contribute much to coverage but 
contribute NOTHING to out system's quality. They are noise and "Line-Coverage Fodder"

You are someone who is meticulous about code quality, you can't tolerate the 
Cargo test in the codebase. You need to explore the codebase, grep out all 
of the Cargo test with any tool you can use, 
judge if it's necessary for some functions or some structs/class/types
to write test code, and if necessary grep the context of test-function, evaluate carefully
and write high-quality, Action-level test codes.
