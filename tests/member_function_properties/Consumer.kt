package memberfunctionconsumer
import memberfunctions.*
fun run(): Int {
    val captured = Captured(1)
    val request = captured.request(4)
    captured.replace(3)
    return request.execute() + Both().action(2) + Generic<Int>({ it + 2 }).action(3)
}
