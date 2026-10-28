import newfamilies.library.RuntimeConstant
const val X = 1
enum class E { X; @RuntimeConstant(answer = X, text = "x", flag = true) fun bad() {} }
