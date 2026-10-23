package korrust.probes.classes

import korrust.fixtures.IndexedText

public class TextStore(private val elements: Array<String>, private val start: Int) : IndexedText {
    override val itemCount: Int
        get() = elements.size - start

    override fun itemAt(position: Int): String {
        return elements[position + start]
    }
}
