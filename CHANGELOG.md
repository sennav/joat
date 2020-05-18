# v0.0.6

- Default timeout is None, configurable per scmd
- Search for config files in home folder when None is available
- Support headers per subcommand
- Do not insert body on POST requests if not specified

# v0.0.5

- Allow request subcommands to override the base endpoint
- Improve debugging messages
- Stream stdout and stderr when executing scripts
- Internal refactoring
- Add rust logging support

# v0.0.4

- Fix typo in config retrieval
- Fix merge priority of configs

# v0.0.3

- Support non json response as text
- Support use of environment variables in vars and allow yaml hashes
- Fix configuration merge
- More friendly message for the auto_complete subcommand
- Update reqwest version
- Add response headers to the template context
- Support reading variables from files as text and json
- Support to forms
- Search config files in any parent folder
- Release script
- Add changelog

# v0.0.2

- Bugfixes

# v0.0.1

- Initial version

