Get-ChildItem -Path "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge" -Recurse -File | Where-Object { $_.FullName -notmatch '\\(node_modules|target|\.git|\.agents|dist)\\' } | Sort-Object LastWriteTime -Descending | Select-Object -First 10 | ForEach-Object { $_.FullName }

