import base58
import unittest
from eth_tx_utils import  make_keccak_instruction_data, Trx
from web3.auto import w3
from solana_utils import *

tokenkeg = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
sysvarclock = "SysvarC1ock11111111111111111111111111111111"
sysinstruct = "Sysvar1nstructions1111111111111111111111111"
keccakprog = "KeccakSecp256k11111111111111111111111111111"
solana_url = os.environ.get("SOLANA_URL", "http://localhost:8899")
http_client = Client(solana_url)
evm_loader_id = os.environ.get("EVM_LOADER")
# evm_loader_id = "6Eo6NybJ45RM62XWzb4eCtdCGCnEELuZm1rSxRM15ocz"
CONTRACTS_DIR = os.environ.get("CONTRACTS_DIR", "evm_loader/")
CONTRACT_ERC20_BIN = CONTRACTS_DIR + "ERC20.binary"
ERC20_CTOR = CONTRACTS_DIR + "ERC20/erc20_ctor_uninit.hex"

class SplToken:
    def __init__(self, url):
        self.url = url

    def call(self, arguments):
        cmd = 'spl-token --url {} {}'.format(self.url, arguments)
        try:
            return subprocess.check_output(cmd, shell=True, universal_newlines=True)
        except subprocess.CalledProcessError as err:
            import sys
            print("ERR: spl-token error {}".format(err))
            raise

def deployERC20(loader, location_hex, location_bin,  mintId, balance_erc20):
    ctor_init = str("%064x" % 0xa0) + \
                str("%064x" % 0xe0) + \
                str("%064x" % 0x9) + \
                base58.b58decode(balance_erc20).hex() + \
                base58.b58decode(mintId).hex() + \
                str("%064x" % 0x1) + \
                str("77%062x" % 0x00) + \
                str("%064x" % 0x1) + \
                str("77%062x" % 0x00)

    with open(location_hex, mode='r') as hex:
        binary = bytearray.fromhex(hex.read() + ctor_init)
        with open(location_bin, mode='wb') as bin:
            bin.write(binary)
            return loader.deploy(location_bin)

class erc20_tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        wallet = WalletAccount(wallet_path())
        cls.loader = EvmLoader(solana_url, wallet, evm_loader_id)
        cls.acc = wallet.get_acc()

        cls.caller_eth_pr_key = w3.eth.account.from_key(cls.acc.secret_key())
        cls.caller_eth = bytes.fromhex(cls.caller_eth_pr_key.address[2:])
        (cls.caller, caller_nonce) = cls.loader.ether2program(cls.caller_eth)

        info = http_client.get_account_info(cls.caller)
        if info['result']['value'] is None:
            print("Create solana caller account...")
            caller = cls.loader.createEtherAccount(cls.caller_eth)
            print("Done")
            print("solana caller:", caller)

        if getBalance(cls.acc.public_key()) == 0:
            print("Create user account...")
            tx = http_client.request_airdrop(cls.acc.public_key(), 10*10**9)
            confirm_transaction(http_client, tx['result'])
            # balance = http_client.get_balance(cls.acc.public_key())['result']['value']
            print("Done\n")

        print('Account:', cls.acc.public_key(), bytes(cls.acc.public_key()).hex())
        print("Caller:", cls.caller_eth.hex(), caller_nonce, "->", cls.caller, "({})".format(bytes(PublicKey(cls.caller)).hex()))

    def createMint(self):
        spl = SplToken(solana_url)
        res = spl.call("create-token")
        if not res.startswith("Creating token "):
            raise Exception("create token error")
        else:
            return res[15:59]

    def createTokenAccount(self, mint_id):
        spl = SplToken(solana_url)
        res = spl.call("create-account {}".format(mint_id))
        if not res.startswith("Creating account "):
            raise Exception("create account error")
        else:
            return res[17:61]

    def changeOwner(self, acc, owner):
        spl = SplToken(solana_url)
        res = spl.call("authorize {} owner {}".format(acc, owner))
        pos = res.find("New owner: ")
        if owner != res[pos+11:pos+55]:
            raise Exception("change owner error")

    def tokenMint(self, mint_id, recipient, amount):
        spl = SplToken(solana_url)
        res = spl.call("mint {} {} {}".format(mint_id, amount, recipient))
        print ("minting {} tokens for {}".format(amount, recipient))

    def tokenBalance(self, acc):
        spl = SplToken(solana_url)
        return int(spl.call("balance {}".format(acc)).rstrip())

    def erc20_deposit(self, payer, amount, erc20, balance_erc20, mint_id, receiver_erc20):
        input = "6f0372af" + \
                base58.b58decode(payer).hex() + \
                str("%024x" % 0) + receiver_erc20.hex() + \
                self.acc.public_key()._key.hex() + \
                "%064x" % amount

        info = getAccountData(http_client, self.caller, ACCOUNT_INFO_LAYOUT.sizeof())
        caller_trx_cnt = int.from_bytes(AccountInfo.frombytes(info).trx_count, 'little')

        trx_raw = { 'to': solana2ether(erc20), 'value': 0, 'gas': 0, 'gasPrice': 0, 'nonce': caller_trx_cnt,
            'data': input, 'chainId': 1 }
        trx_signed = w3.eth.account.sign_transaction(trx_raw, self.caller_eth_pr_key.key)
        trx_parsed = Trx.fromString(trx_signed.rawTransaction)
        trx_rlp = trx_parsed.get_msg(trx_raw['chainId'])
        eth_sig = eth_keys.Signature(vrs=[1 if trx_parsed.v%2==0 else 0, trx_parsed.r, trx_parsed.s]).to_bytes()
        keccak_instruction = make_keccak_instruction_data(1, len(trx_rlp))
        evm_instruction = self.caller_eth + eth_sig + trx_rlp

        trx = Transaction().add(
            TransactionInstruction(program_id=keccakprog, data=keccak_instruction, keys=[
                    AccountMeta(pubkey=PublicKey(keccakprog), is_signer=False,  is_writable=False),  ])).add(
            TransactionInstruction(program_id=self.loader.loader_id,
                                   data=bytearray.fromhex("05") + evm_instruction,
                                   keys=[
                                       AccountMeta(pubkey=erc20, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=self.caller, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=PublicKey(sysinstruct), is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=payer, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=balance_erc20, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=mint_id, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=tokenkeg, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=self.loader.loader_id, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=self.acc.public_key(), is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=PublicKey(sysvarclock), is_signer=False, is_writable=False),
                                   ]))

        result = http_client.send_transaction(trx, self.acc)
        result = confirm_transaction(http_client, result["result"])
        messages = result["result"]["meta"]["logMessages"]
        res = messages[messages.index("Program log: Succeed") + 2]
        if not res.startswith("Program log: "):
            raise Exception("Invalid program logs: no result")
        else:
            if int(res[13:], 16) == 1:
                print ("deposit OK")
            else:
                print ("deposit Fail")


    def erc20_withdraw(self, receiver, amount, erc20, balance_erc20, mint_id):
        input = bytearray.fromhex(
            "441a3e70" +
            base58.b58decode(receiver).hex() +
            "%064x" % amount
        )
        info = getAccountData(http_client, self.caller, ACCOUNT_INFO_LAYOUT.sizeof())
        caller_trx_cnt = int.from_bytes(AccountInfo.frombytes(info).trx_count, 'little')

        trx_raw = { 'to': solana2ether(erc20), 'value': 0, 'gas': 0, 'gasPrice': 0, 'nonce': caller_trx_cnt,
            'data': input, 'chainId': 1 }
        trx_signed = w3.eth.account.sign_transaction(trx_raw, self.caller_eth_pr_key.key)
        trx_parsed = Trx.fromString(trx_signed.rawTransaction)
        trx_rlp = trx_parsed.get_msg(trx_raw['chainId'])
        eth_sig = eth_keys.Signature(vrs=[1 if trx_parsed.v%2==0 else 0, trx_parsed.r, trx_parsed.s]).to_bytes()
        keccak_instruction = make_keccak_instruction_data(1, len(trx_rlp))
        evm_instruction = self.caller_eth + eth_sig + trx_rlp

        trx = Transaction().add(
            TransactionInstruction(program_id=keccakprog, data=keccak_instruction, keys=[
                    AccountMeta(pubkey=PublicKey(keccakprog), is_signer=False,  is_writable=False),  ])).add(
            TransactionInstruction(program_id=self.loader.loader_id,
                                   data=bytearray.fromhex("05") + evm_instruction,
                                   keys=[
                                       AccountMeta(pubkey=erc20, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=self.caller, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=PublicKey(sysinstruct), is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=balance_erc20, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=receiver, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=mint_id, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=tokenkeg, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=self.loader.loader_id, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=self.acc.public_key(), is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=PublicKey(sysvarclock), is_signer=False, is_writable=False),
                                   ]))

        result = http_client.send_transaction(trx, self.acc)
        result = confirm_transaction(http_client, result["result"])
        messages = result["result"]["meta"]["logMessages"]
        res = messages[messages.index("Program log: Succeed") + 2]
        if not res.startswith("Program log: "):
            raise Exception("Invalid program logs: no result")
        else:
            if int(res[13:], 16) == 1:
                print ("wirdraw OK")
            else:
                print ("wirdraw Fail")


    def erc20_balance(self, erc20):
        input = bytearray.fromhex(
            "0370a08231" +
            str("%024x" % 0) + self.caller_eth.hex()
        )
        trx = Transaction().add(
            TransactionInstruction(program_id=self.loader.loader_id, data=input, keys=
            [
                AccountMeta(pubkey=erc20, is_signer=False, is_writable=True),
                AccountMeta(pubkey=self.caller, is_signer=False, is_writable=True),
                AccountMeta(pubkey=self.acc.public_key(), is_signer=True, is_writable=False),
                AccountMeta(pubkey=self.loader.loader_id, is_signer=False, is_writable=False),
                AccountMeta(pubkey=PublicKey(sysvarclock), is_signer=False, is_writable=False),
            ]))

        result = http_client.send_transaction(trx, self.acc)
        result = confirm_transaction(http_client, result["result"])
        messages = result["result"]["meta"]["logMessages"]
        res = messages[messages.index("Program log: Succeed") + 2]
        if not res.startswith("Program log: "):
            raise Exception("Invalid program logs: no result")
        else:
            return int(res[13:], 16)


    def erc20_transfer(self, erc20, eth_to, amount):
        input = bytearray.fromhex(
            "a9059cbb" +
            str("%024x" % 0) + eth_to +
            "%064x" % amount
        )

        info = getAccountData(http_client, self.caller, ACCOUNT_INFO_LAYOUT.sizeof())
        caller_trx_cnt = int.from_bytes(AccountInfo.frombytes(info).trx_count, 'little')

        trx_raw = {'to': solana2ether(erc20), 'value': 0, 'gas': 0, 'gasPrice': 0, 'nonce': caller_trx_cnt,
                   'data': input, 'chainId': 1}
        trx_signed = w3.eth.account.sign_transaction(trx_raw, self.caller_eth_pr_key.key)
        trx_parsed = Trx.fromString(trx_signed.rawTransaction)
        trx_rlp = trx_parsed.get_msg(trx_raw['chainId'])
        eth_sig = eth_keys.Signature(vrs=[1 if trx_parsed.v % 2 == 0 else 0, trx_parsed.r, trx_parsed.s]).to_bytes()
        keccak_instruction = make_keccak_instruction_data(1, len(trx_rlp))
        evm_instruction = self.caller_eth + eth_sig + trx_rlp

        trx = Transaction().add(
            TransactionInstruction(program_id=keccakprog, data=keccak_instruction, keys=[
                AccountMeta(pubkey=PublicKey(keccakprog), is_signer=False, is_writable=False), ])).add(
            TransactionInstruction(program_id=self.loader.loader_id,
                                   data=bytearray.fromhex("05") + evm_instruction,
                                   keys=[
                                       AccountMeta(pubkey=erc20, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=self.caller, is_signer=False, is_writable=True),
                                       AccountMeta(pubkey=PublicKey(sysinstruct), is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=self.loader.loader_id, is_signer=False, is_writable=False),
                                       AccountMeta(pubkey=PublicKey(sysvarclock), is_signer=False, is_writable=False),
                                   ]))
        result = http_client.send_transaction(trx, self.acc)
        result = confirm_transaction(http_client, result["result"])
        messages = result["result"]["meta"]["logMessages"]
        print("erc20 transfer signature: {}".format(result["result"]["transaction"]["signatures"][0]))
        res = messages[messages.index("Program log: Succeed") + 2]
        if not res.startswith("Program log: "):
            raise Exception("Invalid program logs: no result")
        else:
            if int(res[13:], 16) == 1:
                print("transfer OK")
            else:
                print("transfer Fail")

    def erc20_balance_ext(self, erc20):
        input = bytearray.fromhex("0340b6674d")
        trx = Transaction().add(
            TransactionInstruction(program_id=self.loader.loader_id, data=input, keys=
            [
                AccountMeta(pubkey=erc20, is_signer=False, is_writable=True),
                AccountMeta(pubkey=self.caller, is_signer=False, is_writable=True),
                AccountMeta(pubkey=self.acc.public_key(), is_signer=True, is_writable=False),
                AccountMeta(pubkey=self.loader.loader_id, is_signer=False, is_writable=False),
                AccountMeta(pubkey=PublicKey(sysvarclock), is_signer=False, is_writable=False),
            ]))

        result = http_client.send_transaction(trx, self.acc)
        result = confirm_transaction(http_client, result["result"])
        messages = result["result"]["meta"]["logMessages"]
        res = messages[messages.index("Program log: Succeed") + 2]
        if not res.startswith("Program log: "):
            raise Exception("Invalid program logs: no result")
        else:
            return res[13:]


    def erc20_mint_id(self, erc20):
        input = bytearray.fromhex("03e132a122")
        trx = Transaction().add(
            TransactionInstruction(program_id=self.loader.loader_id, data=input, keys=
            [
                AccountMeta(pubkey=erc20, is_signer=False, is_writable=True),
                AccountMeta(pubkey=self.caller, is_signer=False, is_writable=True),
                AccountMeta(pubkey=self.acc.public_key(), is_signer=True, is_writable=False),
                AccountMeta(pubkey=self.loader.loader_id, is_signer=False, is_writable=False),
                AccountMeta(pubkey=PublicKey(sysvarclock), is_signer=False, is_writable=False),
            ]))

        result = http_client.send_transaction(trx, self.acc)
        result = confirm_transaction(http_client, result["result"])
        messages = result["result"]["meta"]["logMessages"]
        res = messages[messages.index("Program log: Succeed") + 2]
        if not res.startswith("Program log: "):
            raise Exception("Invalid program logs: no result")
        else:
            return res[13:]

    def test_erc20(self):
        mintId = self.createMint()
        time.sleep(20)
        print("\ncreate token:", mintId)
        acc_client = self.createTokenAccount(mintId)
        print ("create account acc_client:", acc_client)
        balance_erc20 = self.createTokenAccount(mintId)
        print ("create account balance_erc20:", balance_erc20)

        deploy_result = deployERC20(self.loader, ERC20_CTOR, CONTRACT_ERC20_BIN,  mintId, balance_erc20)
        erc20Id = deploy_result["programId"]
        erc20Id_ether = bytearray.fromhex(deploy_result["ethereum"][2:])

        print ("erc20_id:", erc20Id)
        print ("erc20_id_ethereum:", erc20Id_ether.hex())
        time.sleep(20)
        print("erc20 balance_ext():", self.erc20_balance_ext( erc20Id))
        print("erc20 mint_id():", self.erc20_mint_id( erc20Id))

        self.changeOwner(balance_erc20, erc20Id)
        print("balance_erc20 owner changed to {}".format(erc20Id))
        mint_amount = 100
        self.tokenMint(mintId, acc_client, mint_amount)
        time.sleep(20)
        assert(self.tokenBalance(acc_client) == mint_amount)
        assert(self.tokenBalance(balance_erc20) == 0)
        assert(self.erc20_balance( erc20Id) == 0)

        deposit_amount = 1
        self.erc20_deposit( acc_client,  deposit_amount*(10**9), erc20Id, balance_erc20, mintId, self.caller_eth)
        assert(self.tokenBalance(acc_client) == mint_amount-deposit_amount)
        assert(self.tokenBalance(balance_erc20) == deposit_amount)
        assert(self.erc20_balance( erc20Id) == deposit_amount*(10**9))
        self.erc20_withdraw( acc_client, deposit_amount*(10**9), erc20Id, balance_erc20, mintId)
        assert(self.tokenBalance(acc_client) == mint_amount)
        assert(self.tokenBalance(balance_erc20) == 0)
        assert(self.erc20_balance( erc20Id) == 0)


    @unittest.skip("not for CI")
    def test_deposit(self):
        print("test_deposit")
        acc_client = "297MLscTY5SC4pwpPzTaFQBY4ndHdY1h5jC5FG18RMg2"
        erc20Id = "2a5PhGUpnTsCgVL8TjZ5S3LU76pmUfVC5UBHre4yqs5a"
        balance_erc20= "8VAcZVoXCQoXb74DGMftRpraMYqHK86qKZALmBopo36i"
        mintId = "8y9XyppKvAWyu2Ud4HEAH6jaEAcCCvE53wcmr92t9RJJ"
        receiver_erc20 = bytes.fromhex("0000000000000000000000000000000000000011")
        self.erc20_deposit( acc_client,  900, erc20Id, balance_erc20, mintId, receiver_erc20)

    @unittest.skip("not for CI")
    def test_with_draw(self):
        print("test_withdraw")
        acc_client = "297MLscTY5SC4pwpPzTaFQBY4ndHdY1h5jC5FG18RMg2"
        erc20Id = "2a5PhGUpnTsCgVL8TjZ5S3LU76pmUfVC5UBHre4yqs5a"
        balance_erc20= "8VAcZVoXCQoXb74DGMftRpraMYqHK86qKZALmBopo36i"
        mintId = "8y9XyppKvAWyu2Ud4HEAH6jaEAcCCvE53wcmr92t9RJJ"
        self.erc20_withdraw(acc_client,  10, erc20Id, balance_erc20, mintId)

    @unittest.skip("not for CI")
    def test_balance_ext(self):
        print("test_balance_ext")
        erc20Id = "JDjTbq2CRdpfa12uYcDVHpQXQk5YHcfyrML73z824Uww"
        print(self.erc20_balance_ext( erc20Id))

    @unittest.skip("not for CI")
    def test_mint_id(self):
        print("test_mint_id")
        erc20Id = "JDjTbq2CRdpfa12uYcDVHpQXQk5YHcfyrML73z824Uww"
        print(self.erc20_mint_id( erc20Id))

    @unittest.skip("not for CI")
    def test_balance(self):
        print("test_balance")
        erc20Id = "JDjTbq2CRdpfa12uYcDVHpQXQk5YHcfyrML73z824Uww"
        print(self.erc20_balance( erc20Id))

    @unittest.skip("not for CI")
    def test_tranfer(self):
        print("test_transfer")
        erc20Id = "9EWuA4YE7ABVKQEg1CChcQdozi93w5kLjo8wn3ZB9NKy"
        self.erc20_transfer( erc20Id, "0000000000000000000000000000000000000011", 0)

if __name__ == '__main__':
    unittest.main()

