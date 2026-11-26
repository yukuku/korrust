package constructorvarargsconsumer
import constructorvarargs.Strings
fun invalid(value: Strings) { value.values[0] = "changed" }
