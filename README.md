[![Build Status](https://travis-ci.com/sennav/joat.svg?token=gvqDsu5Cy69X2ywTP4E2&branch=dev)](https://travis-ci.com/sennav/joat)

# Jack of all trades - JOAT

Joat is designed to ease the creation of command line interfaces for REST APIs and
enable automation around these REST APIs.
The program is written in rust and it's pretty much a work in progress, expect errors and breaking changes.
Joat is heavily inspired by [go-jira](https://github.com/go-jira/jira) and uses a lot of powerful rust libraries.

Joat uses a YAML file to define subcommands that can be of two types: requests and scripts.
Requests subcommands ease the interaction with a REST API and scripts are a way of combining multiple commands into a more convenient one.
For instance, suppose you have a team of developers that use trello.com as their Kanban board.
Request commands could be something like `trello get <card_id>`, `trello move <card_id> <column_id>`, `trello assign <card_id> <user>`.
Now suppose a developer needs to perform those three actions to start to working on a card.
One could create a script command like `trello start <card_id>` which always assign oneself to the card and moves it to the in progress column.
All this is configurable in a YAML file that can be shared among all developers to create a useful and tailored CLI for the team.
Joat also combine YAML files from the local folder with files from the home folder, so it's possible to reuse community defined commands and then create your own specific commands on top of those (there's a TODO to make the search for config files recursive).

Some key attributes of this YAML file are treated as templates,
so you can use values defined in environment variables, arguments and more to define what should be send in the request or handled in the script.
The syntax of the templates is Jinja2, you can read more about it [Tera's documentation](https://tera.netlify.com/)
(the rust library used to do this).

## Installation

As this is work in progress there's no packaging of the binaries, it's necessary to compile the rust source code.
To do that you'll need [rust's tools](https://www.rust-lang.org/tools/install) and execute this:

```
git clone <this repo url>
cd joat
cargo build --release
target/release/joat --help
```

As you see after executing these commands the binaries will be at `target/release/joat`.
Joat is not very useful in itself, so you have to create an extension.

## Creating an extension

Just execute:

```
# Create an yaml file with the name of your cli
joat init <name of your cli>
# symlink joat binaries to your cli name (it has to be the same name as the yaml)
ln -s target/release/joat /usr/local/bin/<name of your cli>
# optionally define templates
mkdir templates && touch templates/sample.j2
```

## Config yaml

Sample yaml file:

```
name: gitlab-cli
version: "0.0.1"
author: Vinicius <senna.vmd@gmail.com>
about: Cli to interface with Gitlab's REST API
base_endpoint: https://gitlab.com/api/v4
vars:
    gitlab_project_id: "123"
headers:
    Private-Token: "{{env.GITLAB_TOKEN}}"
args:
    - config:
        short: c
        long: config
        value_name: FILE
        help: Sets a custom config file
        takes_value: true
    - verbose:
        short: v
        multiple: true
        help: Sets the level of verbosity
subcommands:
    - show:
        about: show issue data
        path: /projects/{{env.gitlab_project_id}}/issues/{{args.ISSUE_ID}}
        args:
            - ISSUE_ID:
                help: Id of the issue to show
                required: true
                index: 1
            - template:
                short: t
                long: template
                help: Use a different template
                required: false
                takes_value: true
        response_template: issue.j2
    - show_script:
        about: Sample of a script subcommand
        args:
            - ISSUE_ID:
                help: Id of the issue
                required: true
                index: 1
        script: |
            gitlab show {{args.ISSUE_ID}}
```

## Contributing

Be polite, I'm not earning anything to do this, other than that just create an issue or a PR and we'll take it from there.

## TODO

- [ ] Init subcommand
- [ ] Standard OAuth 2 capabilities
- [ ] Investigate other auth methods
- [ ] Tests
- [ ] Public release
