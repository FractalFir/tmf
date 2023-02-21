First of all,thank you for checking out and deciding to contribute to the tmf project. All contributions are greatly appreciated.
# Before submitting a pull request
1. Ensure that all tests still pass
2. Run `cargo clippy` to see any lint suggestions
3. Run `cargo fmt` to ensure your code is formatted like the rest of the project
4. Run `cargo doc` to ensure documentation has not been accidentally broken.
## For functionality changes
1. compare size of `suzan.tmf`, which appears after tests in `target/test_res` with previous version, and include this size.
2. Import `suzan_ftmf.obj` from `target/test_res` into blender and ensure it sill looks OK.
# What can you contribute?
## Low hanging fruits
There is some work that, while not world-changing and glamorous, is requires almost no experience, and can ins some cases be tackled by someone who does not even know how to write code at all.
### Fixing/Rewording
While the documentation is in a pretty good place ATM and is perfectly usable, there may be some grammatical/spelling mistakes dotted around. I have dyslexia and am not a native speaker, so those kinds of errors can easily slip trough. Pull requests regarding fixes to those kinds of mistakes should be accepted pretty quickly(the are easy to evaluate and will not set the entire project on fire in case of an error). They should be almost always accepted, unless the *mistake* is just a different, accepted spelling(colour vs color), or the change has some different problems(unintentionally changes meaning of documentation).
### Expanding documentation.
While each and every function in documentation has a short description and an example, this can still be greatly improved.
## Slightly more complex stuff
### Refactoring 
As time goes on, amount of code in need of refactoring grows. This can cause issues down the line, so refactoring is greatly appreciated. Because this work requires more skill and time to do, more strict checking of pull requests is needed, and accepting/rejecting them may take more time.
## Any optimisations, bug fixes and other changes that **do not** make the file format incompatible or change the API
For optimisations, as long as all tests still pass, there should be no problems with merging. Additionally, some more test regarding the impact on file sizes should be done. Bug fixes and other changes will be put under more scrutiny. But once again, keep in mind that accepting/rejecting the request may take more time. 
## Very Complex Stuff
### API/Format changes 
I am fairly open to API additions/changes, as long as a reason for inclusion is given(This change will make this easier) and the change is backward-compatible. 
### New features
Any feature that adds anything to the file format will be placed under extreme scrutiny. The TMF project tries to be backward compatible as much as possible, so I want to avoid rolling a feature out only to deprecate it in favour of something way better. 
# More questions?
If you have any more questions, feel free to DM me on reddit, or write me an email. I will gladly answer any questions.
