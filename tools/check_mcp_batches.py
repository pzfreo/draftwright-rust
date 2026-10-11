"""Check the MCP operation limit through HTTP, stdio and serve, without a part load."""
import http.client
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

BINARY = str(Path(sys.argv.pop(1) if len(sys.argv) > 1 else 'target/release/draftwright').resolve())
LIMIT = 32
OPS = [{'op': 'set', 'path': 'sheet/title/title', 'value': 'H5'}] * (LIMIT + 1)


class Batches(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='h5-transports-')
        cls.env = {**os.environ, 'DRAFTWRIGHT_CACHE': cls.directory.name}
        cls.http = subprocess.Popen(
            [BINARY, 'mcp-http', '127.0.0.1:0'], env=cls.env,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
        )
        cls.address = cls.http.stderr.readline().strip().split('http://', 1)[1].removesuffix('/mcp')

    @classmethod
    def tearDownClass(cls):
        cls.http.terminate()
        cls.http.wait(timeout=10)
        cls.http.stderr.close()
        cls.directory.cleanup()

    def rpc(self, transport, method, params):
        request = {'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params}
        if transport == 'http':
            connection = http.client.HTTPConnection(self.address, timeout=30)
            connection.request('POST', '/mcp', json.dumps(request), {
                'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream',
            })
            response = connection.getresponse()
            self.assertEqual(response.status, 200)
            result = json.loads(response.read())
            connection.close()
            return result['result']
        completed = subprocess.run([BINARY, 'mcp-stdio'], env=self.env,
                                   input=json.dumps(request) + '\n', text=True,
                                   capture_output=True, check=True, timeout=30)
        return json.loads(completed.stdout)['result']

    def serve(self, operation, arguments):
        completed = subprocess.run([BINARY, 'serve'], env=self.env,
                                   input=json.dumps({'op': operation, 'arguments': arguments, **arguments}) + '\n',
                                   text=True, capture_output=True, check=True, timeout=30)
        return json.loads(completed.stdout)

    def check_schema(self, transport):
        tools = self.rpc(transport, 'tools/list', {})['tools']
        selected = [tool for tool in tools if tool['name'] in ('preview_ops', 'apply_ops')]
        self.assertEqual({tool['name'] for tool in selected}, {'preview_ops', 'apply_ops'})
        for tool in selected:
            self.assertEqual(tool['inputSchema']['properties']['ops']['maxItems'], LIMIT)
            published = json.loads(Path(f"schemas/mcp/{tool['name']}.input.json").read_text())
            self.assertEqual(tool['inputSchema'], published)

    def check_limit(self, transport):
        for tool in ('preview_ops', 'apply_ops'):
            args = {'part_ref': 'unregistered', 'ops': OPS}
            if tool == 'apply_ops':
                args['parent'] = 'unregistered'
            reply = self.rpc(transport, 'tools/call', {'name': tool, 'arguments': args})
            self.assertTrue(reply['isError'])
            error = reply['structuredContent']['error']
            self.assertEqual(error['code'], 'invalid')
            self.assertIn('at most 32', error['message'])
            self.assertFalse(Path(self.directory.name, 'parts').exists())

    def test_http_published_limit(self):
        self.check_schema('http')

    def test_stdio_published_limit(self):
        self.check_schema('stdio')

    def test_http_rejects_before_part_load(self):
        self.check_limit('http')

    def test_stdio_rejects_before_part_load(self):
        self.check_limit('stdio')

    def test_serve_mcp_rejects_before_part_load(self):
        for tool in ('preview_ops', 'apply_ops'):
            args = {'part_ref': 'unregistered', 'ops': OPS}
            if tool == 'apply_ops':
                args['parent'] = 'unregistered'
            reply = self.serve('mcp_' + tool, args)
            self.assertFalse(reply['ok'])
            self.assertIn('at most 32', reply['error']['message'])

    def test_serve_lossless_rejects_before_part_load(self):
        for operation in ('stateless_preview', 'stateless_apply'):
            reply = self.serve(operation, {'part_ref': 'unregistered', 'ops': OPS})
            self.assertFalse(reply['ok'])
            self.assertIn('at most 32', reply['error']['message'])


if __name__ == '__main__':
    unittest.main()
