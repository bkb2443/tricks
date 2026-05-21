.PHONY: dev stop

NODE := /opt/homebrew/opt/node@20/bin/node
NPM  := /opt/homebrew/opt/node@20/bin/npm

dev:
	@mkdir -p .run
	@echo "Starting server..."
	@cd server && cargo run > ../.run/server.log 2>&1 & echo $$! > ../.run/server.pid
	@echo "Starting client..."
	@cd client && PATH="/opt/homebrew/opt/node@20/bin:$$PATH" $(NPM) run dev > ../.run/client.log 2>&1 & echo $$! > ../.run/client.pid
	@echo "Server log: .run/server.log"
	@echo "Client log: .run/client.log"
	@echo "Client:     http://localhost:5173"
	@echo "Stop with:  make stop"

stop:
	@if [ -f .run/server.pid ]; then kill $$(cat .run/server.pid) 2>/dev/null; rm .run/server.pid; echo "Server stopped"; fi
	@if [ -f .run/client.pid ]; then kill $$(cat .run/client.pid) 2>/dev/null; rm .run/client.pid; echo "Client stopped"; fi

logs:
	@tail -f .run/server.log .run/client.log
