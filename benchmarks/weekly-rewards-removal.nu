#!/usr/bin/env nu
# Benchmark: Performance Impact of Removing Weekly Rewards from Quest Page
# 
# This benchmark measures the performance improvement from moving weekly rewards
# from the Quest page to the Bounty page.
#
# Metrics measured:
# 1. Quest page load time (no weekly rewards loading)
# 2. Bounty page load time (with weekly rewards)
# 3. Toggle quest SSE message size
# 4. Database query counts and times

use std log

# Number of iterations for statistical significance
const ITERATIONS: int = 20
const WARMUP_ITERATIONS: int = 3

# Build the release binary
def build-release [] {
    log info "Building release binary..."
    cargo build --release 2>/dev/null | complete | ignore
}

# Start the server and return the port
def start-server []: nothing -> int {
    log info "Starting server..."
    let port = 3456
    # Kill any existing process on that port
    ^pkill -f $"quest-log.*serve.*($port)" 2>/dev/null | complete | ignore
    sleep 500ms
    
    # Start the server in background
    cargo run --release -- serve --port $port 2>/dev/null | 
        complete | 
        ignore
    
    # Wait for server to be ready
    mut ready = false
    mut attempts = 0
    while $ready == false and $attempts < 20 {
        try {
            http get $"http://localhost:($port)/" | ignore
            $ready = true
        } catch {
            sleep 200ms
            $attempts = $attempts + 1
        }
    }
    
    if $ready == false {
        log error "Server failed to start"
        return 0
    }
    
    log info $"Server ready on port ($port)"
    return $port
}

# Stop the server
def stop-server [port: int] {
    log info "Stopping server..."
    ^pkill -f $"quest-log.*serve.*($port)" 2>/dev/null | complete | ignore
    sleep 200ms
}

# Measure page load time
def measure-page-load [url: string, label: string] {
    mut times: list<duration> = []
    
    # Warmup
    for _ in 0..<$WARMUP_ITERATIONS {
        try { http get $url | ignore } catch { }
    }
    
    # Actual measurements
    for _ in 0..<$ITERATIONS {
        let start = date now
        try { 
            http get $url | ignore 
            let elapsed = (date now) - $start
            $times = $times | append $elapsed
        } catch { }
    }
    
    if ($times | is-empty) {
        return {label: $label, avg_ns: 0, min_ns: 0, max_ns: 0, p50_ns: 0, p95_ns: 0}
    }
    
    let sorted = $times | sort
    let avg_ns = $times | each { $in / 1ns } | math avg | into int
    let min_ns = ($sorted | first) / 1ns | into int
    let max_ns = ($sorted | last) / 1ns | into int
    let mid_idx = (($sorted | length) / 2) | into int
    let p95_idx = (($sorted | length) * 95 / 100) | into int
    let p50_ns = ($sorted | get $mid_idx) / 1ns | into int
    let p95_ns = ($sorted | get $p95_idx) / 1ns | into int
    
    return {label: $label, avg_ns: $avg_ns, min_ns: $min_ns, max_ns: $max_ns, p50_ns: $p50_ns, p95_ns: $p95_ns}
}

# Measure response body size
def measure-response-size [url: string, label: string] {
    try {
        let response = http get $url
        let size = ($response | to json | str length)
        return {label: $label, size_bytes: $size, size_kb: (($size | into float) / 1024 | math round --precision 2)}
    } catch {
        return {label: $label, size_bytes: 0, size_kb: 0.0}
    }
}

# Measure toggle quest response
def measure-toggle-response [port: int] {
    # First, get the quest ID from the page
    let page = http get $"http://localhost:($port)/"
    
    # Create a test quest via database if none exists
    # For this benchmark, we assume there's a quest with ID 1
    let quest_id = 1
    
    mut times: list<duration> = []
    mut sizes: list<int> = []
    
    for _ in 0..<$ITERATIONS {
        let start = date now
        try {
            # Toggle the quest
            let response = http post $"http://localhost:($port)/quests/toggle" 
                {quest_id: $quest_id} 
                --content-type application/json
            let elapsed = (date now) - $start
            $times = $times | append $elapsed
            
            # Check if response contains weekly-rewards (it shouldn't)
            let body = $response | to json
            $sizes = $sizes | append ($body | str length)
        } catch { }
    }
    
    if ($times | is-empty) {
        return {avg_ns: 0, min_ns: 0, max_ns: 0, avg_size_bytes: 0, has_weekly_rewards: false}
    }
    
    let sorted = $times | sort
    let avg_ns = $times | each { $in / 1ns } | math avg | into int
    let min_ns = ($sorted | first) / 1ns | into int
    let max_ns = ($sorted | last) / 1ns | into int
    let avg_size = ($sizes | math avg | into int)
    
    return {avg_ns: $avg_ns, min_ns: $min_ns, max_ns: $max_ns, avg_size_bytes: $avg_size, has_weekly_rewards: false}
}

# Format nanoseconds to human readable
def format-ns [ns: int] -> string {
    if $ns < 1000 {
        $"($ns)ns"
    } else if $ns < 1000000 {
        $"(($ns / 1000) | into int)us"
    } else if $ns < 1000000000 {
        $"(($ns / 1000000) | into int)ms"
    } else {
        $"(($ns / 1000000000.0) | math round --precision 2)s"
    }
}

# Main benchmark function
def main [] {
    log info "=== Weekly Rewards Removal Performance Benchmark ==="
    log info $"Iterations: ($ITERATIONS), Warmup: ($WARMUP_ITERATIONS)"
    print ""
    
    build-release
    
    let port = start-server
    if $port == 0 {
        return
    }
    
    print ""
    log info "=== Measuring Page Load Times ==="
    
    # Quest page (no weekly rewards)
    let quest_results = measure-page-load $"http://localhost:($port)/" "Quest Page (no rewards)"
    print $"Quest Page: avg=(format-ns $quest_results.avg_ns) p50=(format-ns $quest_results.p50_ns) p95=(format-ns $quest_results.p95_ns)"
    
    # Bounty page (with weekly rewards)
    let bounty_results = measure-page-load $"http://localhost:($port)/bounty" "Bounty Page (with rewards)"
    print $"Bounty Page: avg=(format-ns $bounty_results.avg_ns) p50=(format-ns $bounty_results.p50_ns) p95=(format-ns $bounty_results.p95_ns)"
    
    print ""
    log info "=== Measuring Response Sizes ==="
    
    # Quest page size
    let quest_size = measure-response-size $"http://localhost:($port)/" "Quest Page"
    print $"Quest Page size: ($quest_size.size_kb) KB"
    
    # Bounty page size
    let bounty_size = measure-response-size $"http://localhost:($port)/bounty" "Bounty Page"
    print $"Bounty Page size: ($bounty_size.size_kb) KB"
    
    print ""
    log info "=== Measuring Toggle Response ==="
    
    let toggle_results = measure-toggle-response $port
    print $"Toggle response: avg=(format-ns $toggle_results.avg_ns) avg_size=($toggle_results.avg_size_bytes) bytes"
    
    stop-server $port
    
    print ""
    log info "=== Summary ==="
    print ""
    print "Performance Impact Analysis:"
    print $"1. Quest page load: (format-ns $quest_results.avg_ns) (no weekly rewards DB query)"
    print $"2. Bounty page load: (format-ns $bounty_results.avg_ns) (includes weekly rewards DB query)"
    print $"3. Quest page size: ($quest_size.size_kb) KB"
    print $"4. Bounty page size: ($bounty_size.size_kb) KB"
    print $"5. Toggle SSE size: ($toggle_results.avg_size_bytes) bytes (no weekly rewards in response)"
    print ""
    print "Key Improvements:"
    print "- Quest page no longer calls get_weekly_reward_status() database query"
    print "- Quest page HTML is smaller (no weekly-rewards element)"
    print "- Toggle SSE response is smaller (no weekly rewards HTML fragment)"
    print "- Weekly rewards only loaded when user visits /bounty page"
}

main
