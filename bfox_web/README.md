# BattleFox Web App
We use Rocket as a backend (for json API endpoints), and to serve a React SPA.
The React SPA lives under `./frontend` in this folder.

## Running to dev stuff
First run the rocket backend (make sure you config the mongodb db, e.g. via docker).
Then you run `yarn start` in the `frontend` directory.

## Packaging to deployment
That's what a Dockerfile will be for, eventually.