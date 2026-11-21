from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import unquote, urlparse
import json

class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.hrefs = []
    def handle_starttag(self, tag, attrs):
        if tag == 'a':
            self.hrefs.extend(value for key, value in attrs if key == 'href')

root = Path(__file__).resolve().parents[3] / 'target/reports-html'
broken = []
total = 0
pages = 0
for path in root.rglob('*.html'):
    pages += 1
    parser = Links()
    parser.feed(path.read_text())
    for href in parser.hrefs:
        parsed = urlparse(href)
        if parsed.scheme or not parsed.path:
            continue
        total += 1
        if not (path.parent / unquote(parsed.path)).exists():
            broken.append([str(path), href])
print(json.dumps(dict(pages=pages, links_checked=total, broken=broken)))
assert not broken
