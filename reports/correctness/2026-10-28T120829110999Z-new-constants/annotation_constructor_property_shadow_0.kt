import newfamilies.library.RuntimeConstant
const val X = 1
class Holder(val X: Int) { @RuntimeConstant(answer = X, text = "x", flag = true) fun bad() {} }
