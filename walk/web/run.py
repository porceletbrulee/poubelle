import http.server
import socketserver

class Handler(http.server.SimpleHTTPRequestHandler):
	def end_headers(self):
		self.send_header("Cache-Control", "no-cache")
		self.send_header("Expires", "0")
		super().end_headers()

with socketserver.TCPServer(("0.0.0.0", 8000), Handler) as s:
	s.serve_forever()
