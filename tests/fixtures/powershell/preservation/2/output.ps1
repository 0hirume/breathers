$text = @"
first
$(if ($true) { Write-Output text })
"@
Write-Output done
