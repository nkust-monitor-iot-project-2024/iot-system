# stream-extractor

Extract a frame from a video stream (RTSP) in PNG format, and send it to a message broker.

You *should* not start more than 1 instance of this service. If you do, you may receive duplicate frames.
