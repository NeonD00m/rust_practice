param(
    [string]$RepoPath = "."
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $RepoPath)) {
    Write-Error "Path '$RepoPath' does not exist."
    exit 1
}

Push-Location $RepoPath
try {
    $weeks = 52
    $cols = $weeks + 1
    $today = [DateTime]::Today
    $dow = [int]$today.DayOfWeek  # 0 = Sunday, 6 = Saturday
    $totalDays = ($weeks * 7) + $dow
    $startDate = $today.AddDays(-$totalDays)

    # 1. Map grid index -> date string (yyyy-MM-dd)
    $grid = @{}
    $currentDate = $startDate
    for ($i = 0; $i -le $totalDays; $i++) {
        $grid[$i] = $currentDate.ToString("yyyy-MM-dd")
        $currentDate = $currentDate.AddDays(1)
    }

    # 2. Extract commit dates from Git
    $sinceStr = $startDate.ToString("yyyy-MM-dd")
    $gitOutput = git log --since="$sinceStr" --date=short --format=%ad 2>$null

    $counts = @{}
    $maxCount = 0
    if ($gitOutput) {
        foreach ($line in $gitOutput) {
            $d = $line.Trim()
            if ($d) {
                $counts[$d] = [int]$counts[$d] + 1
                if ($counts[$d] -gt $maxCount) {
                    $maxCount = $counts[$d]
                }
            }
        }
    }
    if ($maxCount -eq 0) { $maxCount = 1 }

    # 3. Setup ANSI escape codes and palettes (GitHub greens)
    $esc = [char]27
    $colors = @{
        1 = 22  # dark green
        2 = 28
        3 = 34
        4 = 40  # bright green
    }
    $dayLabels = @("Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat")
    $monthNames = @("", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

    # 4. Construct month header string
    $headerChars = [char[]](" " * ($cols * 2))
    $prevMonth = 0

    for ($col = 0; $col -lt $cols; $col++) {
        $idx = $col * 7
        if ($grid.ContainsKey($idx)) {
            $mon = [int]($grid[$idx].Substring(5, 2))
            if ($mon -ne $prevMonth) {
                $lbl = $monthNames[$mon]
                $pos = $col * 2
                for ($k = 0; $k -lt $lbl.Length; $k++) {
                    if (($pos + $k) -lt $headerChars.Length) {
                        $headerChars[$pos + $k] = $lbl[$k]
                    }
                }
                $prevMonth = $mon
            }
        }
    }

    Write-Host ("    " + (-join $headerChars))

    # 5. Render calendar grid rows
    for ($row = 0; $row -lt 7; $row++) {
        $line = "{0,-4}" -f $dayLabels[$row]
        for ($col = 0; $col -lt $cols; $col++) {
            $idx = ($col * 7) + $row
            if ($grid.ContainsKey($idx)) {
                $dateKey = $grid[$idx]
                $c = if ($counts.ContainsKey($dateKey)) { $counts[$dateKey] } else { 0 }
                if ($c -eq 0) {
                    $line += "  "
                } else {
                    $lvl = [Math]::Floor(($c * 4) / $maxCount) + 1
                    if ($lvl -gt 4) { $lvl = 4 }
                    $colorCode = $colors[[int]$lvl]
                    $line += "$esc[48;5;${colorCode}m  $esc[0m"
                }
            } else {
                $line += "  "
            }
        }
        Write-Host $line
    }
}
finally {
    Pop-Location
}
