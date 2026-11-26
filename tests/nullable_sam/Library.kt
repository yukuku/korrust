package nullable
fun interface IntCallback { fun transform(value: Int): Int }
fun kotlinCall(callback: IntCallback?, value: Int): Int = if (callback == null) -1 else callback.transform(value)
