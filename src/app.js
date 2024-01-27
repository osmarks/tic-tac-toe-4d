import wasm_init from "../pkg/tic_tac_toe_4d"
import "../app.css"
import * as wasm_api from "../pkg/tic_tac_toe_4d"

var worker = new Worker("webworker_glue.js")

win_callback = winner => {
    console.log(winner)
    if ("points" in window) {
        window.points.then(points => {
            console.log("writing to achievement system")
            if (winner === 1) {
                points.unlockAchievement(`ttt4Win`)
                points.updateMetric(`ttt4Wins`, x => x + 1, 0)
            } else if (winner === 2) { 
                points.updateMetric(`ttt4Losses`, x => x + 1, 0)
            } else if (isDraw) {
                points.updateMetric(`ttt4Draws`, x => x + 1, 0)
            }
        })
    }
}

wasm_init().then(x => {
    const callback = wasm_api.main()
    console.log("rust initiated", callback)
    run_ai_background = data => {
        worker.postMessage(data)
        console.log("dispatching")
    }
    
    worker.onmessage = ev => {
        callback(ev.data)
    }
})