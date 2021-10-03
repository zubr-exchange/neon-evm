# File: test_neon_faucet.py
# Test for the faucet service.

import unittest
import os
import io
import time
import subprocess
import requests
from web3 import Web3

issue = 'https://github.com/neonlabsorg/neon-evm/issues/166'
proxy_url = os.environ.get('PROXY_URL', 'http://localhost:9090/solana')
proxy = Web3(Web3.HTTPProvider(proxy_url))
admin = proxy.eth.account.create(issue + '/admin')
user = proxy.eth.account.create(issue + '/user')
proxy.eth.default_account = admin.address

class Test_Neon_Faucet(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        print('\n\n' + issue)

    # @unittest.skip("a.i.")
    def test_neon_faucet_01_eth_token(self):
        print()
        # First request - trigger creation of the account without real transfer
        url = 'http://localhost:{}/request_eth_token'.format(os.environ['FAUCET_RPC_PORT'])
        data = '{"wallet": "' + user.address + '", "amount": 0}'
        r = requests.post(url, data=data)
        if not r.ok:
            print('Response:', r.status_code)
        assert(r.ok)
        # Second request - actual test
        balance_before = proxy.eth.get_balance(user.address)
        print('NEO balance before:', balance_before)
        url = 'http://localhost:{}/request_eth_token'.format(os.environ['FAUCET_RPC_PORT'])
        data = '{"wallet": "' + user.address + '", "amount": 1}'
        r = requests.post(url, data=data)
        if not r.ok:
            print('Response:', r.status_code)
        assert(r.ok)
        # Check
        balance_after = proxy.eth.get_balance(user.address)
        print('NEO balance after:', balance_after)
        print('NEO balance difference:', balance_after - balance_before)
        self.assertEqual(balance_after - balance_before, 1000000000000000000)

    # @unittest.skip("a.i.")
    def test_neon_faucet_02_erc20_tokens(self):
        print()
        a_before = self.get_token_balance(self.token_a, user.address)
        b_before = self.get_token_balance(self.token_b, user.address)
        print('token A balance before:', a_before)
        print('token B balance before:', b_before)
        url = 'http://localhost:{}/request_erc20_tokens'.format(os.environ['FAUCET_RPC_PORT'])
        data = '{"wallet": "' + user.address + '", "amount": 1}'
        r = requests.post(url, data=data)
        if not r.ok:
            print('Response:', r.status_code)
        assert(r.ok)
        a_after = self.get_token_balance(self.token_a, user.address)
        b_after = self.get_token_balance(self.token_b, user.address)
        print('token A balance after:', a_after)
        print('token B balance after:', b_after)
        self.assertEqual(a_after - a_before, 1000000000000000000)
        self.assertEqual(b_after - b_before, 1000000000000000000)

    def get_token_balance(self, token_address, address):
        erc20 = proxy.eth.contract(address=token_address, abi=self.contract['abi'])
        return erc20.functions.balanceOf(address).call()

    @classmethod
    def tearDownClass(cls):
        pass

if __name__ == '__main__':
    unittest.main()
