$paths = @("c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src", "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src", "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\docs", "c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge")
$files = foreach ($p in $paths) {
    if (Test-Path $p) {
        Get-ChildItem -Path $p -File -Recurse -ErrorAction SilentlyContinue
    }
}
$files | Sort-Object LastWriteTime -Descending | Select-Object -First 5 | ForEach-Object { $_.FullName }
