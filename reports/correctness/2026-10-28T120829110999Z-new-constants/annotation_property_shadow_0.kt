import newfamilies.library.RuntimeConstant
const val X = 1
object Holder { val X = 2; @RuntimeConstant(answer = X, text = "x", flag = true) fun bad() {} }
