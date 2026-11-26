import android.view.accessibility.AccessibilityEvent
fun add(event: AccessibilityEvent, value: CharSequence?): Boolean = event.text.add(value)
