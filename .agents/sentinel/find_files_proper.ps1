$files = [System.Collections.Generic.List[System.IO.FileInfo]]::new()
$recursivePaths = @(
    "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src",
    "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src",
    "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\docs"
)
foreach ($p in $recursivePaths) {
    if (Test-Path $p) {
        Get-ChildItem -Path $p -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object { $files.Add($_) }
    }
}
# Non-recursive for root
Get-ChildItem -Path "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge" -File -ErrorAction SilentlyContinue | ForEach-Object { $files.Add($_) }

$files | Sort-Object LastWriteTime -Descending | Select-Object -First 5 | ForEach-Object { $_.FullName }
