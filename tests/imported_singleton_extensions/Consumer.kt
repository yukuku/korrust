package singleton.consumer

import singleton.library.Events
import singleton.library.Words.decorate as finish
import singleton.library.Words.inherited
import singleton.library.Words.head
import singleton.library.Words.orText
import singleton.library.Words.measure
import singleton.library.Words.score
import singleton.library.Host.Named.decorate as finishNamed

fun order(): String = Events.receiver().finish(Events.argument()) + ":" + Events.trace()
fun companionOrder(): String = Events.receiver().finishNamed() + ":" + Events.trace()
fun defaults(): String = "d".finish()
fun named(): String = "d".finish(suffix = "n")
fun overloaded(): String = 7.finish("s")
fun generic(): String = listOf("g").head()
fun nullable(value: String?): String = value.orText("fallback")
fun properties(): Int {
    "abc".score = 20
    return "ab".score + "x".measure
}

fun inherited(): String = "parent".inherited()
