import * as fs from "fs";
import { getSigningClient } from "../util";

const run = async () => {
  const { cw404ContractAddress } = JSON.parse(
    fs.readFileSync("scripts/contract_addresses.json").toString()
  );
  const { signerAddress, siggingClient } = await getSigningClient();

  const tokenId = 1;
  const recipientAddress = signerAddress;

  await siggingClient
    .execute(
      signerAddress,
      cw404ContractAddress,
      {
        send_ft: {
          recipient: recipientAddress,
          token_id: tokenId.toString(),
        },
      },
      "auto",
      "memooooo",
      []
    )
    .then((res) => {
      console.log(res.transactionHash);
    });
};

run();
