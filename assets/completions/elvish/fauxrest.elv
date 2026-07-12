
use builtin;
use str;

set edit:completion:arg-completer[fauxrest] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'fauxrest'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'fauxrest'= {
            cand -L 'Specify the log level'
            cand --level 'Specify the log level'
            cand -c 'Path to the configuration file'
            cand --config 'Path to the configuration file'
            cand -l 'Layout to use for the output'
            cand --layout 'Layout to use for the output'
            cand -d 'Path to the output directory [default: dist]'
            cand --dest 'Path to the output directory [default: dist]'
            cand -s 'Serializer to use for the output. [available: json, typescript, sql] [default: json]'
            cand --serializer 'Serializer to use for the output. [available: json, typescript, sql] [default: json]'
            cand --minify 'If true, minify the output'
            cand --no-minify 'If set, disable minification (overrides config)'
            cand --overwrite 'If true, overwrite existing files in the destination directory'
            cand --copy-static 'Copy all non-JSON static files from the data directory into each destination (allow all). $static exclude globs still take precedence.'
            cand --gencomp 'Generate completion files'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
