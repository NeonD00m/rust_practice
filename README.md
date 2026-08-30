# rust_practice

my goal is just to have fun coding rust without delegating to AI and create mini-side projects which might actually be useful!
*(emphasis on "might")*

usually I focus on minimizing bloat and keeping it to the core necessary features, so file sizes are tiny and performance is never a problem

## projects

#### **todo**
no clever name because I was lazy, though I initially named the exe "mytodo" so that's why it saves to "my_todo.json"

- lightweight task manager focused on fast keyboard workflows
- features quick single-line task creation, **tags** to organize tasks by keywords, and attaching associated files
- paged listing and search filters across task descriptions, tags, and completion status
- `scan` command to import "// TODO:" comments from files and directories (supports a few different languages)

#### **palmtree**
palm trees and beaches are relaxing and I needed this at a high stress time, plus pomodoro is supposedly pronounded "palm"-odoro

- pomodoro timer tool that can do these things
- contains different base settings for DeskTime or ultradian rhythm based timers
- can be hooked up with `todo` by piping the printed task to `palmtree`:
```bash
todo print 0 --completion | palmtree start --wait
```

## introduce yourself to the tools (tutorial)
you gotta install cargo and Rust, you should probably do that with `rustup` if you haven't already

now just start by installing `todo` and running the following commands to get started:
```bash
cargo install --path .\todoCli
todo search intro --files
```



#### Contributing
- please open issues or PRs - small improvements, new features, or bug fixes are definitely welcome
