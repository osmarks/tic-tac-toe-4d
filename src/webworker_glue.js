import wasm_init from "../pkg/tic_tac_toe_4d"
import * as wasm_api from "../pkg/tic_tac_toe_4d"

wasm_init().then(x => {
    onmessage = event => {
        postMessage(wasm_api.run_ai(event.data))
    }
    console.log("background execution ready")
})