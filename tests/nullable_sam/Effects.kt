import nullable.Callbacks
import nullable.kotlinCall
fun direct(): Int = Callbacks.call({ value -> value + 3 }, 4)
fun labeled(): Int = Callbacks.call(callback@{ value -> if (value == 4) return@callback 12; 0 }, 4)
fun capture(): Int { var total = 3; val result = Callbacks.call({ value -> total += value; total }, 4); return result + total }
fun absent(): Int = Callbacks.call(null, 4)
fun constructor(): Int = Callbacks { value -> value + 9 }.invoke(4)
fun generic(): String = Callbacks.map({ value -> value + "!" }, "ok")
fun nullableArgument(value: String?): Int { var total = 1; Callbacks.event({ text -> if (text == null) total = 8 else total = text.length }, value); return total }
fun ordinaryNullableFunction(callback: ((Int) -> Int)?): Int = if (callback == null) -1 else callback(4)
fun ordinaryLiteral(): Int = ordinaryNullableFunction { value -> value + 2 }
fun kotlinCallback(): Int = kotlinCall({ value -> value + 4 }, 5)
fun callbackThrows(): Int = Callbacks.call({ _ -> throw IllegalStateException("callback") }, 4)
