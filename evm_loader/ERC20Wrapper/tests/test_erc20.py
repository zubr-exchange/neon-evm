#!/usr/bin/env python3
#----------------------------------------
# Call methods of an ERC20 smart contract.
#----------------------------------------
# Environment variables:
# WEB3_RPC_URL - contains URL to access the Ethereum network (example: http://localhost:8545)
# WEB3_ADMIN_KEY - contains private key of the contract deployer
# WEB3_ADMIN - contains Ethereum address (EOA) of the contract deployer
# WEB3_USER_KEY -  - contains private key of a user
# WEB3_USER - contains Ethereum address (EOA) of a user
# WEB3_ADDRESS - contains Ethereum address of the ERC20 smart contract
# WEB3_ABI_FILE - contains name of file in which the contract's ABI is stored
#----------------------------------------------------------------------------

from os import environ
from web3 import Web3

def check_env(name):
    if not name in environ:
        raise Exception(name)

check_env('WEB3_RPC_URL')
check_env('WEB3_ADMIN_KEY')
check_env('WEB3_ADMIN')
check_env('WEB3_USER_KEY')
check_env('WEB3_USER')
check_env('WEB3_ADDRESS')
check_env('WEB3_ABI_FILE')

url = environ.get('WEB3_RPC_URL')
admin_key = environ.get('WEB3_ADMIN_KEY')
admin = environ.get('WEB3_ADMIN')
user_key = environ.get('WEB3_USER_KEY')
user = environ.get('WEB3_USER')
contract_address = environ.get('WEB3_ADDRESS')

w3 = Web3(Web3.HTTPProvider(url))
with open(environ.get('WEB3_ABI_FILE'), 'r') as file:
    abi = file.read()
erc20 = w3.eth.contract(address=contract_address, abi=abi)

print('Name:', erc20.functions.name().call())
print('Symbol:', erc20.functions.symbol().call())
print('Decimals:', erc20.functions.decimals().call())
print('Admin:', admin)
print('User:', user)

# Supply admin
nonce = w3.eth.get_transaction_count(admin)
tx = {
    'nonce': nonce,
    'from': admin
}
tx = erc20.functions.transfer(admin, 100000000000000000000).buildTransaction(tx)
signed_tx = w3.eth.account.sign_transaction(tx, admin_key)
tx_hash = w3.eth.send_raw_transaction(signed_tx.rawTransaction)
tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
assert tx_receipt != None

print()
print('Testing totalSupply()...')
total_supply = erc20.functions.totalSupply().call()
print('totalSupply =', total_supply)
assert total_supply == 10000000000000000000000000000000000000

print()
print('Testing balanceOf(user)...')
balance0 = erc20.functions.balanceOf(user).call()
print('balanceOf(user) is', balance0)

print()
print('Testing transfer(user,1000)...')
nonce = w3.eth.get_transaction_count(admin)
tx = {
    'nonce': nonce,
    'from': admin
}
tx = erc20.functions.transfer(user, 1000).buildTransaction(tx)
signed_tx = w3.eth.account.sign_transaction(tx, admin_key)
tx_hash = w3.eth.send_raw_transaction(signed_tx.rawTransaction)
tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
assert tx_receipt != None
balance1 = erc20.functions.balanceOf(user).call()
print('balanceOf(user) is', balance1)
diff = balance1 - balance0
print('Balance of user changed:', diff)
assert diff == 1000

print()
print('Testing approve(user,100000)...')
nonce = w3.eth.get_transaction_count(admin)
tx = {
    'nonce': nonce,
    'from': admin
}
tx = erc20.functions.approve(user, 100000).buildTransaction(tx)
signed_tx = w3.eth.account.sign_transaction(tx, admin_key)
tx_hash = w3.eth.send_raw_transaction(signed_tx.rawTransaction)
tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
assert tx_receipt != None

print()
print('Testing allowance(admin,user)...')
allowance = erc20.functions.allowance(admin, user).call()
print('allowance is', allowance)
assert allowance == 100000

print()
print('Testing transferFrom(admin,user,1000)...')
balance0 = erc20.functions.balanceOf(user).call()
nonce = w3.eth.get_transaction_count(user)
tx = {
    'nonce': nonce,
    'from': user
}
tx = erc20.functions.transferFrom(admin, user, 1000).buildTransaction(tx)
signed_tx = w3.eth.account.sign_transaction(tx, user_key)
tx_hash = w3.eth.send_raw_transaction(signed_tx.rawTransaction)
tx_receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
assert tx_receipt != None
balance1 = erc20.functions.balanceOf(user).call()
diff = balance1 - balance0
print('Balance of user changed:', diff)
assert diff == 1000

print('\nDone.')
