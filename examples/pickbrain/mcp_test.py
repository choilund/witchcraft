import subprocess
import json
import sys
import os

def send_request(proc, request):
    line = json.dumps(request) + "\n"
    proc.stdin.write(line.encode())
    proc.stdin.flush()
    resp_line = proc.stdout.readline().decode().strip()
    if not resp_line:
        return None
    return json.loads(resp_line)

def main():
    # Use the installed binary or custom path
    binary = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/.cargo/bin/pickbrain")
    if not os.path.exists(binary):
        print(f"Error: {binary} not found.")
        sys.exit(1)

    print(f"Starting MCP server: {binary}")
    # Force PICKBRAIN_DB to a temp location if needed, but for now we test current db
    proc = subprocess.Popen(
        [binary, "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=0
    )

    try:
        # 1. Initialize
        print("Sending 'initialize' request...")
        init_req = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            }
        }
        resp = send_request(proc, init_req)
        if resp is None:
            raise Exception("No response from server")
            
        print(f"Response: {json.dumps(resp, indent=2)}")
        
        assert resp["id"] == 1
        assert "result" in resp
        assert "protocolVersion" in resp["result"]
        print("SUCCESS: 'initialize' protocol correct.")

        # 2. List tools
        print("\nSending 'tools/list' request...")
        list_req = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }
        resp = send_request(proc, list_req)
        if resp is None:
            raise Exception("No response from server")
            
        print(f"Found tools: {[t['name'] for t in resp['result']['tools']]}")
        
        assert resp["id"] == 2
        tools = [t["name"] for t in resp["result"]["tools"]]
        assert "search_ai_history" in tools
        assert "read_session" in tools
        print(f"SUCCESS: Found {len(tools)} tools.")

        print("\nMCP Integration Test PASSED")

    except Exception as e:
        print(f"\nFAILED: {e}")
        # Print stderr if something went wrong
        stderr_output = proc.stderr.read().decode()
        if stderr_output:
            print(f"Server stderr:\n{stderr_output}")
        sys.exit(1)
    finally:
        proc.terminate()

if __name__ == "__main__":
    main()
