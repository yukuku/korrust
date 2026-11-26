package binarydefaults.compose
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Constraints
fun measure(measurer: TextMeasurer, text: AnnotatedString, style: TextStyle, constraints: Constraints): TextLayoutResult =
    measurer.measure(text, style, constraints = constraints)
fun factory(): Constraints = Constraints(maxWidth = 160)
