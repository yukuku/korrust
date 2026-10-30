import kotlin.test.assertEquals
fun box(): String {
    val x = safecast<String>("abc")
    assertEquals("abc", x)
    val y = safecast<Int>(1)
    assertEquals(1, y)

    val z = safecast<Int>("abc")
    assertEquals(null, z)

    return "OK"
}
