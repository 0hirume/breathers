#Requires -Version 7.0
using namespace System.IO
function Get-Value {
    param([string]$Path)
    begin { Write-Output ready }

    process {
        $value = Get-Item $Path

        # attached
        if ($value) {
            Write-Output $value
        }

        Get-Process
        | Sort-Object CPU

        RETURN $value
    }

    clean { Remove-Variable value }
}

Write-Output done
