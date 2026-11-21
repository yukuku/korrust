package fieldmeta
import fieldmeta.dependency.Choice
import fieldmeta.dependency.JavaMode
fun first(): Choice = Choice.FIRST
fun pick(flag: Boolean): Choice = if (flag) Choice.FIRST else Choice.SECOND
fun route(value: Choice): Int = when (value) {
    Choice.FIRST -> 1
    Choice.SECOND -> 2
}

fun javaConstant(): JavaMode = JavaMode.TYPE_USE
