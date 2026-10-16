param(
    [Parameter(Mandatory = $false, Position = 0)] [string]
	$Username,

    [Parameter(Mandatory = $false)]
    [string]$Token =$env:GITHUB_TOKEN
)

$ErrorActionPreference = "Stop"

# 1. Resolve Auth Token
if (-not $Token) {
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $Token = (gh auth token 2>$null)
    }
}

if (-not $Token) {
    Write-Error @"
Authentication token required.
Run 'gh auth login' or pass -Token 'ghp_...'
"@
    exit 1
}

# 2. Build GraphQL Query (requests native contributionLevel)
$query = if ($Username) {
@"
query(`$user: String!) {
  user(login: `$user) {
    contributionsCollection {
      contributionCalendar {
        totalContributions
        weeks {
          contributionDays {
            date
            contributionCount
            contributionLevel
            weekday
          }
        }
      }
    }
  }
}
"@
} else {
@"
query {
  viewer {
    login
    contributionsCollection {
      contributionCalendar {
        totalContributions
        weeks {
          contributionDays {
            date
            contributionCount
            contributionLevel
            weekday
          }
        }
      }
    }
  }
}
"@
}

$bodyObj = @{ query =$query }
if ($Username) {
    $bodyObj["variables"] = @{ user = $Username }
}
$body =$bodyObj | ConvertTo-Json

$headers = @{
    "Authorization" = "Bearer $Token"
    "User-Agent"    = "PowerShell-GitHub-Calendar"
}

Write-Host "Querying GitHub API..." -ForegroundColor Cyan

try {
    $response = Invoke-RestMethod -Uri "https://api.github.com/graphql" `
                                  -Method Post `
                                  -Headers $headers `
                                  -Body $body `
                                  -ContentType "application/json"
} catch {
    Write-Error "Failed to query GitHub API: $_"
    exit 1
}

if ($response.errors) {
    Write-Error "GraphQL Error: $($response.errors[0].message)"
    exit 1
}

$userData = if ($Username) { $response.data.user } else {$response.data.viewer }
if (-not $userData) {
    Write-Error "Could not retrieve user data."
    exit 1
}

$calendar =$userData.contributionsCollection.contributionCalendar
$total =$calendar.totalContributions
$targetUser = if ($Username) { $Username } else {$userData.login }

# 3. Setup Grid Data & Native 256-Color Palette
$weeks =$calendar.weeks
$cols =$weeks.Count
$esc = [char]27

# 256-color ANSI codes supported across Windows Terminal and conhost
$levelColors = @{
    "NONE"            = "  "
    "FIRST_QUARTILE"  = "$esc[48;5;28m$esc[0m"  # Dark forest green
    "SECOND_QUARTILE" = "$esc[48;5;34m$esc[0m"  # Medium green
    "THIRD_QUARTILE"  = "$esc[48;5;40m$esc[0m"  # Bright green
    "FOURTH_QUARTILE" = "$esc[48;5;46m$esc[0m"  # Neon green
}

$dayLabels = @("Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat")
$monthNames = @("", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

$dayMap = @{}
for ($c = 0; $c -lt $cols; $c++) {
    foreach ($day in $weeks[$c].contributionDays) {
        $wDay = [int]$day.weekday
        $dayMap["$c,$wDay"] = @{
            Date  = $day.date
            Count = [int]$day.contributionCount
            Level = [string]$day.contributionLevel
        }
    }
}

# 4. Build Month Header
$headerChars = [char[]](" " * ($cols * 2))
$prevMonth = 0

for ($c = 0; $c -lt $cols; $c++) {
    $firstDay =$null
    for ($r = 0; $r -lt 7; $r++) {
        if ($dayMap.ContainsKey("$c,$r")) {
            $firstDay =$dayMap["$c,$r"].Date
            break
        }
    }

    if ($firstDay) {
        $mon = [int]($firstDay.Substring(5, 2))
        if ($mon -ne $prevMonth) {$lbl = $monthNames[$mon]
            $pos =$c * 2
            for ($k = 0; $k -lt $lbl.Length; $k++) {
                if (($pos +$k) -lt $headerChars.Length) {$headerChars[$pos +$k] = $lbl[$k]
                }
            }
            $prevMonth =$mon
        }
    }
}

Write-Host ""
Write-Host "User: $targetUser" -ForegroundColor Cyan
Write-Host "Total contributions in the last 52 weeks: $total" -ForegroundColor Green
Write-Host ""
Write-Host ("    " + (-join $headerChars))

# 5. Render Grid
for ($row = 0; $row -lt 7; $row++) {$line = "{0,-4}" -f $dayLabels[$row]
    for ($col = 0; $col -lt$cols; $col++) {$key = "$col,$row"
        if ($dayMap.ContainsKey($key)) {$dayInfo = $dayMap[$key]
            $lvl =$dayInfo.Level
            $cnt =$dayInfo.Count

            if ($lvl -and $levelColors.ContainsKey($lvl)) {
                # Fallback: if GitHub marked NONE but count > 0, bump to lowest green tier
                if ($lvl -eq "NONE" -and $cnt -gt 0) {
                    $line +=$levelColors["FIRST_QUARTILE"]
                } else {
                    $line += $levelColors[$lvl]
                }
            } elseif ($cnt -gt 0) {
                $line +=$levelColors["FIRST_QUARTILE"]
            } else {
                $line += "  "
            }
        } else {
            $line += "  "
        }
    }
    Write-Host $line
}
Write-Host ""
