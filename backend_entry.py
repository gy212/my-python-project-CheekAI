import uvicorn
import os
import sys

# Add the parent directory to sys.path to ensure imports work correctly
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

if __name__ == '__main__':
    # Import here to avoid issues if imports happen at top level before sys.path fix
    try:
        if getattr(sys, 'frozen', False):
            # If running as compiled exe, set config dir to be relative to executable
            # We placed config in 'backend/config' relative to dist root
            app_dir = os.path.dirname(sys.executable)
            config_dir = os.path.join(app_dir, 'backend', 'config')
            os.environ['CHEEKAI_CONFIG_DIR'] = config_dir
            print(f"Running in frozen mode. Config dir: {config_dir}")

        from backend.app.main import api
        # Run the server
        # Workers=1 is standard for this type of local app
        uvicorn.run(api, host="127.0.0.1", port=8787, log_level="info")
    except Exception as e:
        print(f"Failed to start server: {e}")
        import traceback
        traceback.print_exc()
        input("Press Enter to exit...")
