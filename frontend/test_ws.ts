import WebSocket from "ws";
import { Command, Response } from "@lirays/scada-proto";

const ws = new WebSocket("ws://localhost:1236");
ws.binaryType = "arraybuffer";

ws.on("open", () => {
  console.log("connected");

  // First let's LIST the root
  const listCmd = Command.encode({
    list: {
      cmdId: "123",
    },
  }).finish();

  ws.send(listCmd);
});

ws.on("message", (data) => {
  const resp = Response.decode(new Uint8Array(data as ArrayBuffer));
  console.log(JSON.stringify(resp, null, 2));

  if (resp.list) {
    console.log("Got list! Trying to GET a var if any.");
    const varIds = Object.values(resp.list.childrenVars).map((v) => v.varId);
    if (varIds.length > 0) {
      const getCmd = Command.encode({
        get: {
          cmdId: "124",
          varIds,
        },
      }).finish();
      ws.send(getCmd);
    } else {
      // let's create a var
      console.log("No vars. ADDing one.");
      const addCmd = Command.encode({
        add: {
          cmdId: "125",
          parentId: Object.values(resp.list.childrenFolders)[0] || "root", // Assuming there is a root or something
          itemsMeta: [
            { name: "test_var", iType: 2, varDType: 1 }, // ITEM_TYPE_VARIABLE, VAR_DATA_TYPE_INTEGER
          ],
        },
      }).finish();
      ws.send(addCmd);
    }
  }

  if (resp.add) {
    console.log("Added! Let's LIST again.");
    const listCmd = Command.encode({
      list: { cmdId: "126" },
    }).finish();
    ws.send(listCmd);
  }

  if (resp.get) {
    console.log("Got GET! Value: ", resp.get.varValues);

    // Let's SET it
    const setCmd = Command.encode({
      set: {
        cmdId: "127",
        varIdsValues: [
          {
            varId: Object.values(resp.get.varValues)[0]?.value
              ? "test_var"
              : "test_var",
            value: { integerValue: 42 },
          },
        ], // Need actual var_id here
      },
    }).finish();
    ws.send(setCmd);
  }

  if (resp.set) {
    console.log("SET success! Let's GET again.");
    // We need the actual varId...
    // But wait, the previous GET would show it. Let's just exit for now.
    process.exit(0);
  }
});
