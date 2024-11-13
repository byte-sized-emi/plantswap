package app.plantswap.frontend

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material.Button
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.preat.peekaboo.image.picker.SelectionMode
import com.preat.peekaboo.image.picker.rememberImagePickerLauncher
import com.preat.peekaboo.ui.camera.PeekabooCamera
import com.preat.peekaboo.ui.camera.rememberPeekabooCameraState

@Composable
fun MultiImageCapturer(
    onNewImages: (List<ByteArray>) -> Unit,
    capturingImage: Boolean,
    onCapturingImageChange: (Boolean) -> Unit,
) {

    val scope = rememberCoroutineScope()
    val singleImagePicker = rememberImagePickerLauncher(
        selectionMode = SelectionMode.Multiple(maxSelection = 5),
        scope = scope,
        onResult = { byteArrays ->
            onNewImages(byteArrays)
        }
    )

    val cameraState = rememberPeekabooCameraState(onCapture = {
        if (it != null) {
            onNewImages(listOf(it))
        }
        onCapturingImageChange(false)
    })

    if (capturingImage) {
        Box(modifier = Modifier.fillMaxSize()) {
            PeekabooCamera(
                state = cameraState,
                modifier = Modifier.fillMaxSize(),
                permissionDeniedContent = {
                    Text("Permission denied")
                },
            )
            Button(
                modifier = Modifier.align(Alignment.BottomCenter),
                onClick = { cameraState.capture() }
            ) {
                Text("Capture")
            }
        }
    } else {
        Button(onClick = { onCapturingImageChange(true) }) {
            Text("Capture image")
        }
        Button(onClick = { singleImagePicker.launch() }) {
            Text("Pick image")
        }
    }

}
