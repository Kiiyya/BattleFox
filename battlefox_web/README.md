# BattleFox Web App
We use Rocket as a backend (for json API endpoints), and to serve a React SPA.
The React SPA lives under `./frontend` in this folder.

## Running to dev stuff
<!-- First run the rocket backend (make sure you config the mongodb db, e.g. via docker).
Then you run `yarn start` in the `frontend` directory. -->
Idk man.

But remember to put the following in your `.env` located in `/battlefox_web/` (port as specified in
`/docker-compose-development.yml`):
```
MONGO_DB=mongodb://localhost:27017/
```

## Packaging to deployment
Eventually deployment will be just CI/CD.

In the repo root, *not* in this folder, run the following:
```
/ $ docker build -t kiiya/battlefox_web:latest -f .\battlefox_web\Dockerfile .
```