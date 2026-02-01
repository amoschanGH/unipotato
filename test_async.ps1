# Test async behavior by making concurrent requests
# If server is async: both complete in ~3 seconds
# If server is sync: both complete in ~6 seconds (sequential)

Write-Host "Starting async test..." -ForegroundColor Cyan
Write-Host "Making 2 concurrent requests to /api/slow (3 second delay each)" -ForegroundColor Cyan
Write-Host ""

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

# Start both requests concurrently
$job1 = Start-Job -ScriptBlock { 
    $start = Get-Date
    Invoke-RestMethod -Uri "http://localhost:8000/api/slow" -Method GET
    $end = Get-Date
    "Request 1 took: $(($end - $start).TotalSeconds) seconds"
}

$job2 = Start-Job -ScriptBlock { 
    $start = Get-Date
    Invoke-RestMethod -Uri "http://localhost:8000/api/slow" -Method GET
    $end = Get-Date
    "Request 2 took: $(($end - $start).TotalSeconds) seconds"
}

# Also make a quick request while slow ones are running
Start-Sleep -Milliseconds 500
Write-Host "Making a quick request to /api/health while slow requests are in progress..." -ForegroundColor Yellow
$quickStart = Get-Date
$quickResult = Invoke-RestMethod -Uri "http://localhost:8000/api/health" -Method GET
$quickEnd = Get-Date
Write-Host "Quick request completed in $(($quickEnd - $quickStart).TotalMilliseconds)ms" -ForegroundColor Green
Write-Host "Response: $($quickResult | ConvertTo-Json -Compress)" -ForegroundColor Gray

# Wait for slow jobs
$results = Wait-Job $job1, $job2 | Receive-Job

$stopwatch.Stop()

Write-Host ""
Write-Host "=== Results ===" -ForegroundColor Cyan
$results | ForEach-Object { Write-Host $_ }
Write-Host ""
Write-Host "Total time for both requests: $($stopwatch.Elapsed.TotalSeconds) seconds" -ForegroundColor Cyan
Write-Host ""

if ($stopwatch.Elapsed.TotalSeconds -lt 5) {
    Write-Host "✅ ASYNC CONFIRMED! Requests ran concurrently (< 5 seconds total)" -ForegroundColor Green
} else {
    Write-Host "❌ Server appears to be blocking (> 5 seconds = sequential processing)" -ForegroundColor Red
}

# Cleanup
Remove-Job $job1, $job2
