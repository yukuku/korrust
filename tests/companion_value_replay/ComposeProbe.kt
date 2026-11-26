package companionvalues.compose
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
fun color(): Color = Color.Red
fun direct(builder: AnnotatedString.Builder) { builder.addStyle(SpanStyle(color = Color.Red), 0, 6) }
fun implicit(): AnnotatedString = buildAnnotatedString { addStyle(SpanStyle(background = Color.Yellow), 0, 6) }
