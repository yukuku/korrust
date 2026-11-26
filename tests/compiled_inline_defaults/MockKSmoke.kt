package defaults.mocking

import io.mockk.every
import io.mockk.mockk

interface Sample {
    fun count(): Int
}

fun relaxed(): Int {
    val sample = mockk<Sample>(relaxed = true)
    val initial = sample.count()
    every { sample.count() } returns 17
    return initial * 100 + sample.count()
}

fun strict(): Int {
    val sample = mockk<Sample>()
    every { sample.count() } returns 19
    return sample.count()
}
