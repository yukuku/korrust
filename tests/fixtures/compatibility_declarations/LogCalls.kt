package compatibility.declarations

import yuku.alkitab.datatransfer.process.LogInterface

fun appLogger(prefix: String): (String) -> Unit {
    val target = LogInterface { line -> Effects.mark(line + ";") }
    return target.logger(prefix)
}

fun appCapturedLog(prefix: String): String {
    var text = ""
    val target = LogInterface { line -> text += "$line;" }
    val log = target.logger(prefix)
    log("alpha")
    log("beta")
    return text
}
