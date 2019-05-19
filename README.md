# Jack of all trades - JOAT

Joat is an experiment to ease the creation of command line interfaces for REST APIs.
The program is written in rust and it's pretty much a work in progress, excpect errors and breaking changes.

Joat uses a yaml file to define subcommands that can be of two types: requests and scripts.
Some key attributes of this yaml file are treated as templates,
you can use values defined in environment variables, arguments and more to define what should be send in the request.
The syntax of the templates is Jinja2, you can read more about it [Tera's documentation](https://tera.netlify.com/)
(the rust library used to do this).

The request subcommand type contains details like endpoint and HTTP verb.
You can define arguments that can go in the url and in the request body.
You may use the templating system to define the request should be performed.

The script subcommand executes a bash script defined in the yaml file.
This type of script is useful when there's a need to combine multiple endpoints to perform meaningful actions.
Scripts also make use of the templating system.

## Instalation

As this is work in progress there's no packaging of the binaries, it's necessary to compile the rust source code.
To do that you'll need [rust's tools](https://www.rust-lang.org/tools/install) and execute this:

```
git clone <this repo url>
cd joat
cargo build --release
target/release/joat --help
```

As you see after executing this the binaries will be at `target/release/joat`.
Joat is not very useful in itself, one of the commands being considered is `joat install <joat-extension>`
which should install yaml files and templates for a extension that has specifics for a particular REST API.

## Creating an extension

Right now there's only one sample extension being defined as the example implementation, it's for gitlab's API.
Here are the steps to get an extenstion working:

```
# Create an yaml file with the name of your cli
touch gitlab.yml
# alias or symlink joat binaries to your cli name (it has to be the same name as the yaml)
alias gitlab=<absolute path to joat binaries>
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

## TODO

- [ ] Install subcommand
- [ ] Init subcommand
- [ ] Script does not inherit shell variables and aliases
- [ ] OAuth 2 capabilities
- [ ] Investigate other auth methods
- [ ] Insert template absolute path on script context (allow calling other lang scripts).