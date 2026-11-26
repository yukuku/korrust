import android.content.Intent
fun literal(): Intent { val result = Intent(); result.putExtra("key", "literal"); return result }
fun nullable(value: Any?): Intent { val result = Intent(); result.putExtra("key", value.toString()); return result }
