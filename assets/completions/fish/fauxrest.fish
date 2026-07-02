complete -c fauxrest -s L -l level -d 'Specify the log level' -r -f -a "error\t''
warn\t''
info\t''
debug\t''
trace\t''"
complete -c fauxrest -s c -l config -d 'Path to the configuration file' -r -F
complete -c fauxrest -s l -l layout -d 'Layout to use for the output' -r -f -a "index\t'Outputs endpoints as `/endpoint/index.[ext]`. Highly compatible with all static web servers, maintaining clean URLs'
file\t'Outputs endpoints as extensionless files (`/endpoint`). **Smart Fallback Specification**: To avoid physical file-directory collisions, collections that contain sub-paths are automatically replaced (fallback) by `.../index.[ext]` files during compilation'
extension\t'Outputs endpoints with explicit extensions (`/endpoint.[ext]`). 100% web server compatible'"
complete -c fauxrest -s d -l dest -d 'Path to the output directory' -r -F
complete -c fauxrest -s s -l serializer -d 'Serializer to use for the output. [available: json, typescript, sql]' -r
complete -c fauxrest -l minify -d 'If true, minify the output'
complete -c fauxrest -s h -l help -d 'Print help (see more with \'--help\')'
complete -c fauxrest -s V -l version -d 'Print version'
