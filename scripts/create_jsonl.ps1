# Script PowerShell para crear archivo JSONL
1..20 | ForEach-Object { 
    "{`"value`": $_}" 
} | Out-File -FilePath "scripts\data\sample.jsonl" -Encoding utf8

