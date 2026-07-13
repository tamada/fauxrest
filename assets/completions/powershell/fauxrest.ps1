
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'fauxrest' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'fauxrest'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'fauxrest' {
            [CompletionResult]::new('-L', '-L ', [CompletionResultType]::ParameterName, 'Specify the log level')
            [CompletionResult]::new('--level', '--level', [CompletionResultType]::ParameterName, 'Specify the log level')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to the configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to the configuration file')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Layout to use for the output')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'Layout to use for the output')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Path to the output directory [default: dist]')
            [CompletionResult]::new('--dest', '--dest', [CompletionResultType]::ParameterName, 'Path to the output directory [default: dist]')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Serializer to use for the output. [available: json, typescript, sql] [default: json]')
            [CompletionResult]::new('--serializer', '--serializer', [CompletionResultType]::ParameterName, 'Serializer to use for the output. [available: json, typescript, sql] [default: json]')
            [CompletionResult]::new('--minify', '--minify', [CompletionResultType]::ParameterName, 'If true, minify the output')
            [CompletionResult]::new('--no-minify', '--no-minify', [CompletionResultType]::ParameterName, 'If set, disable minification (overrides config)')
            [CompletionResult]::new('--overwrite', '--overwrite', [CompletionResultType]::ParameterName, 'If true, overwrite existing files in the destination directory')
            [CompletionResult]::new('--copy-static', '--copy-static', [CompletionResultType]::ParameterName, 'Copy all non-JSON static files from the data directory into each destination (allow all). $static exclude globs still take precedence.')
            [CompletionResult]::new('--gencomp', '--gencomp', [CompletionResultType]::ParameterName, 'Generate completion files')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
