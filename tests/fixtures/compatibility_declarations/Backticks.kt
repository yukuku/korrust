package compatibility.declarations

import org.junit.Test

class DescriptiveTest {
    @Test
    fun `adds two numbers`() {
        if (`when`(4) != 5) throw IllegalStateException("keyword call")
        Effects.mark("B")
    }

    fun `when`(value: Int): Int = value + 1
    fun plain(value: Int): Int = `plain helper`(value)
    fun `plain helper`(value: Int): Int = value + 2
}

fun escapedCall(value: Int): Int = DescriptiveTest().`when`(value)
