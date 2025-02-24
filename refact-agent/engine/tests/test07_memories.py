import asyncio
import aiohttp
import json
from refact import chat_client

base_url = "http://127.0.0.1:8001/v1"


async def test_memory_operations():
    m0 = await chat_client.mem_add(base_url, "seq-of-acts", "compile", "proj1", "Wow, running cargo build on proj1 was successful!")
    m1 = await chat_client.mem_add(base_url, "proj-fact", "compile", "proj1", "Looks like proj1 is written in fact in Rust.")
    m2 = await chat_client.mem_add(base_url, "seq-of-acts", "compile", "proj2", "Wow, running cargo build on proj2 was successful!")
    m3 = await chat_client.mem_add(base_url, "proj-fact", "compile", "proj2", "Looks like proj2 is written in fact in Rust.")
    print("Added memories:\n%s\n%s\n%s\n%s" % (m0, m1, m2, m3))

    bl, bl_t = await chat_client.mem_block_until_vectorized(base_url)
    print("Block result: %0.1fs %s" % (bl_t, bl))

    update_result = await chat_client.mem_update_used(base_url, m1["memid"], +1, -1)
    print("Updated memory:", update_result)

    erase_result = await chat_client.mem_erase(base_url, m0["memid"])
    print("Erased memory:", erase_result)

    http_status, query_result = await chat_client.mem_query(base_url, "compile", "proj1")
    print("Query result: %s\n%s" % (http_status, json.dumps(query_result, indent=4)))


if __name__ == "__main__":
    asyncio.run(test_memory_operations())
