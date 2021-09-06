import base58
import rlp
import json
from web3 import Web3
from web3.auto.gethdev import w3
from solana.rpc.api import Client

client = Client("https://api.devnet.solana.com")
minimal_tx = None
minimal_slot = client.get_slot()["result"]
counter = 0
skip_first = 0
continue_table = {}
holder_table = {}
while (True):
    result = client.get_signatures_for_address("eeLSJgWzzxrqKv1UxtRVVH8FX3qCQWUs9QuAjJpETGU", before=minimal_tx)
    if len(result["result"]) == 0:
        print("len(result[\"result\"]) == 0")
        break
    else:
        print("NEW ITERATION")
    for tx in result["result"]:
        if counter % 100 == 0:
            print( counter )
        if tx["slot"] < minimal_slot:
            minimal_slot = tx["slot"]
            minimal_tx = tx["signature"]
        if counter > skip_first:
            trx = client.get_confirmed_transaction(tx["signature"])
            # print(json.dumps(trx, indent=4, sort_keys=True))
            if trx['result']['transaction']['message']['instructions'] is not None:
                for instruction in trx['result']['transaction']['message']['instructions']:
                    instruction_data = base58.b58decode(instruction['data'])

                    if instruction_data[0] == 0x00: # Write
                        print("{:>6} Write {}".format(counter, len(instruction_data)))

                        offset = int.from_bytes(instruction_data[4:8], "little")
                        length = int.from_bytes(instruction_data[8:16], "little")
                        data = instruction_data[16:]

                        write_account = trx['result']['transaction']['message']['accountKeys'][instruction['accounts'][0]]

                        if write_account in holder_table:
                            for index in range(length):
                                holder_table[write_account][1][1+offset+index] = data[index]

                            signature = holder_table[write_account][1][1:66]
                            length = int.from_bytes(holder_table[write_account][1][66:74], "little")
                            unsigned_msg = holder_table[write_account][1][74:74+length]

                            try:
                                eth_trx = rlp.decode(unsigned_msg)

                                eth_trx[6] = int(signature[64]) + 35 + 2 * int.from_bytes(eth_trx[6], "little")
                                eth_trx[7] = signature[:32]
                                eth_trx[8] = signature[32:64]

                                # print(rlp.encode(eth_trx).hex())
                                eth_signature = '0x' + bytes(Web3.keccak(rlp.encode(eth_trx))).hex()

                                from_address = w3.eth.account.recover_transaction(rlp.encode(eth_trx).hex())

                                storage_account = holder_table[write_account][0]

                                if storage_account in continue_table:
                                    (logs, status, gas_used, return_value, block_number, block_hash) = continue_table[storage_account]
                                    result = {
                                        "transactionHash": eth_signature,
                                        "transactionIndex": hex(0),
                                        "blockHash": block_hash,
                                        "blockNumber": block_number,
                                        "from": from_address,
                                        "to": '0x' + eth_trx[3].hex(),
                                        "gasUsed": '0x%x' % gas_used,
                                        "cumulativeGasUsed": '0x%x' % gas_used,
                                        "contractAddress": '0x' + eth_trx[3].hex(),
                                        "logs": logs,
                                        "status": status,
                                        "logsBloom": "0x"+'0'*512
                                    }
                                    print(json.dumps(result, indent=4))

                                    del continue_table[storage_account]
                                else:
                                    print("Storage not found")
                                    print(eth_signature, "unknown")
                                    # raise

                                del holder_table[write_account]
                            except Exception as err:
                                print("could not parse trx", err)
                                pass
                        else:
                            print("write_account not found")

                    if instruction_data[0] == 0x01: # Finalize
                        print("{:>6} Finalize 0x{}".format(counter, instruction_data.hex()))

                    if instruction_data[0] == 0x02: # CreateAccount
                        print("{:>6} CreateAccount 0x{}".format(counter, instruction_data[-21:-1].hex()))

                    if instruction_data[0] == 0x03: # Call
                        print("{:>6} Call 0x{}".format(counter, instruction_data.hex()))

                    if instruction_data[0] == 0x04: # CreateAccountWithSeed
                        print("{:>6} CreateAccountWithSeed 0x{}".format(counter, instruction_data.hex()))

                    if instruction_data[0] == 0x05: # CallFromRawTrx
                        print("{:>6} CallFromRawTrx 0x{}".format(counter, instruction_data.hex()))

                        collateral_pool_buf = instruction_data[1:5]
                        from_addr = instruction_data[5:25]
                        sign = instruction_data[25:90]
                        unsigned_msg = instruction_data[90:]

                        eth_trx = rlp.decode(unsigned_msg)
                        eth_trx[6] = int(sign[64]) + 35 + 2 * int.from_bytes(eth_trx[6], "little")
                        eth_trx[7] = sign[:32]
                        eth_trx[8] = sign[32:64]

                        # print(rlp.encode(eth_trx).hex())
                        eth_signature = '0x' + bytes(Web3.keccak(rlp.encode(eth_trx))).hex()

                        from_address = w3.eth.account.recover_transaction(rlp.encode(eth_trx).hex())

                        block_number = hex(trx['result']['slot'])
                        block_hash = '0x%064x'%trx['result']['slot']
                        got_result = False
                        logs = []
                        status = "0x1"
                        gas_used = 0
                        return_value = None
                        log_index = 0
                        for inner in (trx['result']['meta']['innerInstructions']):
                            for event in inner['instructions']:
                                log = base58.b58decode(event['data'])
                                instruction_data = log[:1]
                                if (int().from_bytes(instruction_data, "little") == 7):  # OnEvent evmInstruction code
                                    address = log[1:21]
                                    count_topics = int().from_bytes(log[21:29], 'little')
                                    topics = []
                                    pos = 29
                                    for _ in range(count_topics):
                                        topic_bin = log[pos:pos + 32]
                                        topics.append('0x'+topic_bin.hex())
                                        pos += 32
                                    data = log[pos:]
                                    rec = { 'address': '0x'+address.hex(),
                                            'topics': topics,
                                            'data': '0x'+data.hex(),
                                            'transactionLogIndex': hex(0),
                                            'transactionIndex': hex(inner['index']),
                                            'blockNumber': block_number,
                                            # 'transactionHash': trxId,
                                            'logIndex': hex(log_index),
                                            'blockHash': block_hash
                                        }
                                    logs.append(rec)
                                    log_index +=1
                                elif int().from_bytes(instruction_data, "little") == 6:  # OnReturn evmInstruction code
                                    got_result = True
                                    if log[1] < 0xd0:
                                        status = "0x1"
                                    else:
                                        status = "0x0"
                                    gas_used = int.from_bytes(log[2:10], 'little')
                                    return_value = log[10:]

                        result = {
                            "transactionHash": eth_signature,
                            "transactionIndex": hex(0),
                            "blockHash": block_hash,
                            "blockNumber": block_number,
                            "from": from_address,
                            "to": '0x' + eth_trx[3].hex(),
                            "gasUsed": '0x%x' % gas_used,
                            "cumulativeGasUsed": '0x%x' % gas_used,
                            "contractAddress": '0x' + eth_trx[3].hex(),
                            "logs": logs,
                            "status": status,
                            "logsBloom": "0x"+'0'*512
                        }
                        print(json.dumps(result, indent=4))

                    if instruction_data[0] == 0x09: # PartialCallFromRawEthereumTX
                        print("{:>6} PartialCallFromRawEthereumTX 0x{}".format(counter, instruction_data.hex()))

                        collateral_pool_buf = instruction_data[1:5]
                        step_count = instruction_data[5:13]
                        from_addr = instruction_data[13:33]
                        sign = instruction_data[33:98]
                        unsigned_msg = instruction_data[98:]

                        eth_trx = rlp.decode(unsigned_msg)
                        eth_trx[6] = int(sign[64]) + 35 + 2 * int.from_bytes(eth_trx[6], "little")
                        eth_trx[7] = sign[:32]
                        eth_trx[8] = sign[32:64]

                        # print(rlp.encode(eth_trx).hex())
                        eth_signature = '0x' + bytes(Web3.keccak(rlp.encode(eth_trx))).hex()

                        from_address = w3.eth.account.recover_transaction(rlp.encode(eth_trx).hex())
                        print(from_address, from_addr)

                        storage_account = trx['result']['transaction']['message']['accountKeys'][instruction['accounts'][0]]

                        if storage_account in continue_table:
                            (logs, status, gas_used, return_value, block_number, block_hash) = continue_table[storage_account]
                            result = {
                                "transactionHash": eth_signature,
                                "transactionIndex": hex(0),
                                "blockHash": block_hash,
                                "blockNumber": block_number,
                                "from": from_addr,
                                "to": '0x' + eth_trx[3].hex(),
                                "gasUsed": '0x%x' % gas_used,
                                "cumulativeGasUsed": '0x%x' % gas_used,
                                "contractAddress": '0x' + eth_trx[3].hex(),
                                "logs": logs,
                                "status": status,
                                "logsBloom": "0x"+'0'*512
                            }
                            print(json.dumps(result, indent=4))

                            del continue_table[storage_account]
                        else:
                            print("Storage not found")
                            raise

                    if instruction_data[0] == 0x0a: # Continue
                        print("{:>6} Continue 0x{}".format(counter, instruction_data.hex()))

                        block_number = hex(trx['result']['slot'])
                        block_hash = '0x%064x'%trx['result']['slot']
                        got_result = False
                        logs = []
                        status = "0x1"
                        gas_used = 0
                        return_value = None
                        log_index = 0
                        for inner in (trx['result']['meta']['innerInstructions']):
                            for event in inner['instructions']:
                                log = base58.b58decode(event['data'])
                                instruction_data = log[:1]
                                if (int().from_bytes(instruction_data, "little") == 7):  # OnEvent evmInstruction code
                                    address = log[1:21]
                                    count_topics = int().from_bytes(log[21:29], 'little')
                                    topics = []
                                    pos = 29
                                    for _ in range(count_topics):
                                        topic_bin = log[pos:pos + 32]
                                        topics.append('0x'+topic_bin.hex())
                                        pos += 32
                                    data = log[pos:]
                                    rec = { 'address': '0x'+address.hex(),
                                            'topics': topics,
                                            'data': '0x'+data.hex(),
                                            'transactionLogIndex': hex(0),
                                            'transactionIndex': hex(inner['index']),
                                            'blockNumber': block_number,
                                            # 'transactionHash': trxId,
                                            'logIndex': hex(log_index),
                                            'blockHash': block_hash
                                        }
                                    logs.append(rec)
                                    log_index +=1
                                elif int().from_bytes(instruction_data, "little") == 6:  # OnReturn evmInstruction code
                                    got_result = True
                                    if log[1] < 0xd0:
                                        status = "0x1"
                                    else:
                                        status = "0x0"
                                    gas_used = int.from_bytes(log[2:10], 'little')
                                    return_value = log[10:]

                        if got_result:
                            # print(json.dumps(trx['result']['transaction']['message']['accountKeys'], indent=4))
                            # print(json.dumps(instruction['accounts'], indent=4))
                            storage_account = trx['result']['transaction']['message']['accountKeys'][instruction['accounts'][0]]
                            continue_table[storage_account] = (logs, status, gas_used, return_value, block_number, block_hash)

                    if instruction_data[0] == 0x0b: # ExecuteTrxFromAccountDataIterative
                        print("{:>6} ExecuteTrxFromAccountDataIterative 0x{}".format(counter, instruction_data.hex()))

                        holder_account = trx['result']['transaction']['message']['accountKeys'][instruction['accounts'][0]]
                        storage_account = trx['result']['transaction']['message']['accountKeys'][instruction['accounts'][1]]

                        if holder_account in holder_table:
                            print("holder_account found")
                            print("ERRRRRRRRRRRRRRRRRRR")
                            holder_table[holder_account] = (storage_account, bytearray(128*1024))
                        else:
                            holder_table[holder_account] = (storage_account, bytearray(128*1024))
                            
                    if instruction_data[0] == 0x0c: # Cancel
                        print("{:>6} Cancel 0x{}".format(counter, instruction_data.hex()))

                        storage_account = trx['result']['transaction']['message']['accountKeys'][instruction['accounts'][0]]
                        # continue_table[storage_account] = 0xff
                        continue_table[storage_account] = (None, None, None, None, None, None)
        counter += 1
print("total tx", counter)