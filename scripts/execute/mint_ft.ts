import * as fs from "fs";
import { getSigningClient } from "../util";

const run = async () => {
  const { cw404ContractAddress } = JSON.parse(
    fs.readFileSync("scripts/contract_addresses.json").toString()
  );
  const { signerAddress, siggingClient } = await getSigningClient();

  const mintAmount = 5_500_000;
  await siggingClient
    .execute(
      signerAddress,
      cw404ContractAddress,
      {
        mint_ft: {
          amount: mintAmount.toString(),
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
