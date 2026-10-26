package compatibility.declarations

fun chainedAlias(value: Caption): Label = label(value)
fun callbackAlias(value: Int): Int = aliasApply(value) { item -> item + 5 }
fun invokeGoto(action: OnGotoFinished) { action(GotoTab(1), 2, 3, 4) }
